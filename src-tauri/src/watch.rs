//! Market watch: my listings against the cheapest online competition every few
//! minutes, and price alerts from the warframe.market new-orders feed. Both
//! report through notify::send; matching a price is always a click in the app.

use crate::notify::{self, Notice};
use crate::AppState;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::sync::LazyLock;
use std::time::Duration;
use tauri::{AppHandle, Manager};
use wfc_core::market::Order;

#[derive(Debug, Clone, Serialize)]
pub struct WatchRow {
    pub order_id: String,
    pub item_id: String,
    pub slug: String,
    pub name: String,
    /// "sell" or "buy"
    pub kind: String,
    pub mine: u32,
    pub quantity: u32,
    pub rank: Option<u8>,
    pub visible: bool,
    /// Cheapest online seller (sell) / highest online buyer (buy), others only.
    pub best: Option<u32>,
    pub best_user: Option<String>,
    pub beaten: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceAlert {
    pub slug: String,
    pub name: String,
    pub item_id: String,
    /// Tell me when it is listed at this or less.
    pub max: u32,
}

#[derive(Default, Clone, Serialize)]
pub struct Snapshot {
    pub checked: i64,
    pub rows: Vec<WatchRow>,
}

static LAST: LazyLock<Mutex<Snapshot>> = LazyLock::new(Default::default);
/// "order:price" pairs and alert order ids already announced.
static TOLD: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(Default::default);

pub fn last() -> Snapshot {
    LAST.lock().clone()
}

pub fn alerts(st: &AppState) -> Vec<PriceAlert> {
    st.db.get("price_alerts").and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

fn online(o: &Order) -> bool {
    o.visible && o.user.as_ref().is_some_and(|u| u.status == "ingame" || u.status == "online")
}

/// Compare every own listing with the market (one request per item, spaced
/// for the 3 req/s rule) and announce new undercuts / outbids.
pub async fn check(app: &AppHandle) -> anyhow::Result<Snapshot> {
    let st = app.state::<AppState>();
    anyhow::ensure!(st.market.token().is_some(), "log in to warframe.market first");
    let mine = st.market.my_orders().await?;
    let me = st.db.get("wfm_name").unwrap_or_default().to_lowercase();
    let items = st.market.items();
    let mut rows = vec![];
    for o in &mine {
        let Some(item) = items.iter().find(|i| i.id == o.item_id) else { continue };
        let mut row = WatchRow {
            order_id: o.id.clone(),
            item_id: o.item_id.clone(),
            slug: item.slug.clone(),
            name: item.name.clone(),
            kind: o.kind.clone(),
            mine: o.platinum,
            quantity: o.quantity,
            rank: o.rank,
            visible: o.visible,
            best: None,
            best_user: None,
            beaten: false,
        };
        if o.visible {
            tokio::time::sleep(Duration::from_millis(400)).await;
            match st.market.orders(&item.slug).await {
                Ok(orders) => {
                    let rivals = orders.iter().filter(|x| {
                        x.kind == o.kind
                            && x.rank == o.rank
                            && online(x)
                            && x.user.as_ref().is_some_and(|u| u.ingame_name.to_lowercase() != me)
                    });
                    let best = if o.kind == "sell" { rivals.min_by_key(|x| x.platinum) } else { rivals.max_by_key(|x| x.platinum) };
                    if let Some(b) = best {
                        row.best = Some(b.platinum);
                        row.best_user = b.user.as_ref().map(|u| u.ingame_name.clone());
                        row.beaten = if o.kind == "sell" { b.platinum < o.platinum } else { b.platinum > o.platinum };
                    }
                }
                Err(e) => tracing::warn!("watch {}: {e}", item.slug),
            }
        }
        if row.beaten && TOLD.lock().insert(format!("{}:{}", row.order_id, row.best.unwrap_or(0))) {
            let who = row.best_user.clone().unwrap_or_default();
            let (title, verb) = if row.kind == "sell" { ("Undercut", "sells") } else { ("Outbid", "buys") };
            let n = Notice::new(
                "undercut",
                format!("{title}: {}", row.name),
                format!("{who} {verb} at {}p, you're at {}p", row.best.unwrap_or(0), row.mine),
            )
            .item(&st, &row.slug);
            notify::send(app, n);
        }
        rows.push(row);
    }
    let snap = Snapshot { checked: chrono::Utc::now().timestamp(), rows };
    *LAST.lock() = snap.clone();
    Ok(snap)
}

/// Background check while logged in; `watch_listings` = "0" turns it off,
/// `watch_minutes` sets the interval (default 5, at least 2).
pub fn spawn(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(60)).await;
        loop {
            let st = app.state::<AppState>();
            let on = st.db.get("watch_listings").as_deref() != Some("0");
            let mins = st.db.get("watch_minutes").and_then(|m| m.parse::<u64>().ok()).unwrap_or(5).max(2);
            if on && st.market.token().is_some() {
                if let Err(e) = check(&app).await {
                    tracing::warn!("market watch: {e}");
                }
            }
            tokio::time::sleep(Duration::from_secs(mins * 60)).await;
        }
    });
}

/// A new order from the socket feed: announce it when it hits a price alert.
pub fn on_new_order(app: &AppHandle, payload: &Value) {
    let Ok(o) = serde_json::from_value::<Order>(payload.clone()) else { return };
    if o.kind != "sell" || !online(&o) {
        return;
    }
    let st = app.state::<AppState>();
    let Some(a) = alerts(&st).into_iter().find(|a| a.item_id == o.item_id && o.platinum <= a.max) else { return };
    if !TOLD.lock().insert(o.id.clone()) {
        return;
    }
    let who = o.user.as_ref().map(|u| u.ingame_name.as_str()).unwrap_or("someone");
    let n = Notice::new("price", format!("{} for {}p", a.name, o.platinum), format!("{who} just listed it (alert at {}p or less)", a.max))
        .item(&st, &a.slug);
    notify::send(app, n);
}

// ---- Riven sniper (#11) ----

/// A saved riven search: weapon, positive stats that must all be there, and
/// the most I would pay.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RivenSnipe {
    pub weapon: String,
    pub name: String,
    #[serde(default)]
    pub positives: Vec<String>,
    #[serde(default)]
    pub negative: Option<String>,
    pub max: u32,
}

pub fn snipes(st: &AppState) -> Vec<RivenSnipe> {
    st.db.get("riven_snipes").and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

fn snipe_matches(s: &RivenSnipe, a: &wfc_core::riven::RivenAuction) -> bool {
    a.buyout.is_some_and(|p| p <= s.max)
        && s.positives.iter().all(|p| a.attributes.iter().any(|t| t.positive && &t.url_name == p))
        && s.negative.as_ref().is_none_or(|n| a.attributes.iter().any(|t| !t.positive && &t.url_name == n))
}

/// Every 5 minutes, one search per saved snipe, two seconds apart (the site
/// asks for gentle contract searches). Auctions seen on the first pass of a
/// snipe are not announced; only new ones are.
pub fn spawn_sniper(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut seen: HashSet<String> = HashSet::new();
        let mut first: HashSet<String> = HashSet::new();
        loop {
            tokio::time::sleep(Duration::from_secs(90)).await;
            let st = app.state::<AppState>();
            for s in snipes(&st) {
                match wfc_core::riven::riven_auctions(&st.http, &s.weapon).await {
                    Ok(list) => {
                        let fresh = first.insert(format!("{}|{:?}|{:?}|{}", s.weapon, s.positives, s.negative, s.max));
                        for a in list.iter().filter(|a| snipe_matches(&s, a)) {
                            if seen.insert(a.id.clone()) && !fresh {
                                let stats: Vec<&str> = a.attributes.iter().filter(|t| t.positive).map(|t| t.url_name.as_str()).collect();
                                let mut n = Notice::new(
                                    "riven",
                                    format!("{} {} for {}p", s.name, a.name, a.buyout.unwrap_or(0)),
                                    format!("{} · {} rerolls · {}", stats.join(", ").replace('_', " "), a.rerolls, a.owner),
                                );
                                n.url = Some(format!("https://warframe.market/auction/{}", a.id));
                                notify::send(&app, n);
                            }
                        }
                    }
                    Err(e) => tracing::warn!("riven sniper {}: {e}", s.weapon),
                }
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
            tokio::time::sleep(Duration::from_secs(210)).await;
        }
    });
}