use crate::AppState;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};
use wfc_core::db::{Datapoint, InvRow, RelicRun, Trade};
use wfc_core::export::Export;
use wfc_core::market::{MarketItem, Order, PriceSummary};
use wfc_core::mastery::MasterySummary;
use wfc_core::relic::RelicPlan;
use wfc_core::riven::{RivenGrade, StatInput};

type R<T> = Result<T, String>;
fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

fn export(st: &AppState) -> R<Arc<Export>> {
    st.export.read().clone().ok_or_else(|| "export not loaded yet".into())
}

#[derive(Serialize)]
pub struct Status {
    pub export_loaded: bool,
    pub market_items: usize,
    pub wfm_logged_in: bool,
    pub player: Option<String>,
    pub ee_log: String,
    pub ee_log_exists: bool,
}

#[tauri::command]
pub fn status(st: State<AppState>) -> Status {
    let p = wfc_core::ee_log_path();
    Status {
        export_loaded: st.export.read().is_some(),
        market_items: st.market.items().len(),
        wfm_logged_in: st.market.token().is_some(),
        player: st.player.read().clone().or_else(|| st.db.get("player_name")),
        ee_log_exists: p.exists(),
        ee_log: p.display().to_string(),
    }
}

#[tauri::command]
pub async fn refresh_data(st: State<'_, AppState>) -> R<()> {
    st.market.load_items().await.map_err(err)?;
    let _ = std::fs::remove_file(Export::cache_path());
    let ex = Export::fetch(&st.http).await.map_err(err)?;
    *st.export.write() = Some(Arc::new(ex));
    Ok(())
}

#[tauri::command]
pub fn set_setting(st: State<AppState>, key: String, value: String) -> R<()> {
    st.db.set(&key, &value).map_err(err)
}
#[tauri::command]
pub fn get_setting(st: State<AppState>, key: String) -> Option<String> {
    st.db.get(&key)
}

#[tauri::command]
pub async fn load_profile(st: State<'_, AppState>, player_id: String) -> R<MasterySummary> {
    let player_id = player_id.trim().trim_matches('"').to_string();
    let p = wfc_core::profile::fetch(&st.http, &player_id).await.map_err(err)?;
    st.db.set("player_id", &player_id).map_err(err)?;
    if !p.display_name.is_empty() {
        st.db.set("player_name", &p.display_name).map_err(err)?;
    }
    st.db.set("profile_cache", &serde_json::to_string(&p).map_err(err)?).map_err(err)?;
    let ex = export(&st)?;
    Ok(wfc_core::mastery::summary(&ex, Some(&p)))
}

#[derive(Serialize)]
pub struct SyncResult {
    pub platinum: i64,
    pub credits: i64,
    pub ducats: i64,
    pub items_total: usize,
    pub items_tradeable: usize,
    pub seconds: f32,
}

/// Read the inventory from the game process and replace the local ledger.
#[tauri::command]
pub async fn inventory_sync(app: AppHandle) -> R<SyncResult> {
    crate::overlay::sync_inventory(&app).await.map_err(err)
}

/// Shows the overlay with real prices for a few sample rewards, so the HUD can
/// be checked without cracking a relic.
#[tauri::command]
pub async fn overlay_preview(app: AppHandle) -> R<()> {
    use wfc_core::ocr::RewardHit;
    let st = app.state::<AppState>();
    let samples = ["Wisp Prime Systems Blueprint", "Forma Blueprint", "Nikana Prime Blade", "Braton Prime Receiver"];
    let mut hits = Vec::new();
    for raw in samples {
        let price = match st.market.find_item(raw) {
            Some(mi) => st.market.price(&mi.slug).await.ok(),
            None => None,
        };
        hits.push(RewardHit { raw: raw.to_string(), price, best: false });
    }
    if let Some(i) = hits
        .iter()
        .enumerate()
        .max_by_key(|(_, h)| h.price.as_ref().and_then(|p| p.sell).unwrap_or(0))
        .map(|(i, _)| i)
    {
        hits[i].best = true;
    }
    use tauri::Emitter;
    app.emit("rewards", &hits).map_err(err)?;
    crate::overlay::show(&app);
    Ok(())
}

#[tauri::command]
pub fn mastery(st: State<AppState>) -> R<MasterySummary> {
    let ex = export(&st)?;
    let p = st.db.get("profile_cache").and_then(|s| serde_json::from_str(&s).ok());
    Ok(wfc_core::mastery::summary(&ex, p.as_ref()))
}

#[derive(Serialize)]
pub struct RelicHit {
    pub relic: String,
    pub rarity: String,
    pub chance: [f32; 4],
    pub vaulted: bool,
}

#[tauri::command]
pub fn relics_for(st: State<AppState>, item: String) -> R<Vec<RelicHit>> {
    let ex = export(&st)?;
    Ok(wfc_core::relic::relics_for(&ex, &item)
        .into_iter()
        .map(|(relic, rarity, chance)| {
            let vaulted = st.market.find_item_exact(&format!("{relic} Relic")).map(|i| i.vaulted).unwrap_or(false);
            RelicHit { relic, rarity, chance, vaulted }
        })
        .collect())
}

#[tauri::command]
pub fn relic_list(st: State<AppState>) -> R<Vec<String>> {
    let ex = export(&st)?;
    let mut v: Vec<String> = ex.relics.iter().map(|r| r.name.clone()).collect();
    v.sort();
    Ok(v)
}

#[tauri::command]
pub async fn relic_plan(st: State<'_, AppState>, relic: String) -> R<RelicPlan> {
    let ex = export(&st)?;
    let r = ex
        .relics
        .iter()
        .find(|r| r.name.eq_ignore_ascii_case(&relic))
        .ok_or("unknown relic")?;
    Ok(wfc_core::relic::plan(&ex, &st.market, r).await)
}

#[tauri::command]
pub fn market_search(st: State<AppState>, q: String, limit: usize) -> Vec<MarketItem> {
    let n = wfc_core::norm(&q);
    let mut v: Vec<MarketItem> = st
        .market
        .items()
        .into_iter()
        .filter(|i| wfc_core::norm(&i.name).contains(&n))
        .collect();
    v.sort_by_key(|i| i.name.len());
    v.truncate(limit);
    v
}

/// Resolve warframe.market item ids (as used in orders) to catalogue entries.
#[tauri::command]
pub fn market_items_by_id(st: State<AppState>, ids: Vec<String>) -> std::collections::HashMap<String, MarketItem> {
    let items = st.market.items();
    ids.into_iter()
        .filter_map(|id| items.iter().find(|i| i.id == id).map(|i| (id, i.clone())))
        .collect()
}

/// Is the Warframe process up right now (setup checklist).
#[tauri::command]
pub fn game_running() -> bool {
    wfc_core::memory::find_pid().is_some()
}

/// Is the game window in front right now (the overlay hides otherwise).
#[tauri::command]
pub fn game_focused() -> bool {
    let pid = wfc_core::memory::find_pid();
    pid.is_some() && wfc_core::memory::foreground_pid() == pid
}

/// Latest world state (cycles, fissures, Baro …), refreshed once a minute.
#[tauri::command]
pub fn worldstate(st: State<AppState>) -> Option<serde_json::Value> {
    st.worldstate.read().clone()
}

/// Turn the overlay's edit mode on or off (also bound to the hotkey).
#[tauri::command]
pub fn overlay_edit(app: AppHandle, on: bool) {
    crate::overlay::set_edit(&app, on);
}

/// Errors and warnings from the UI, into the same log file as the backend.
#[tauri::command]
pub fn log_frontend(level: String, message: String) {
    match level.as_str() {
        "error" => tracing::error!(target: "ui", "{message}"),
        "warn" => tracing::warn!(target: "ui", "{message}"),
        _ => tracing::info!(target: "ui", "{message}"),
    }
}

/// Zip a description, the app logs, the settings (tokens removed) and optionally
/// the end of EE.log onto the desktop, and show it in Explorer.
#[tauri::command]
pub fn export_bug_report(app: AppHandle, st: State<AppState>, description: String, include_game_log: bool) -> R<String> {
    use tauri_plugin_opener::OpenerExt;
    tracing::info!("bug report requested: {description}");
    let version = app.package_info().version.to_string();
    let path = crate::diag::write_report(&crate::diag::Report {
        version: &version,
        description: &description,
        include_game_log,
        game_running: wfc_core::memory::find_pid().is_some(),
        settings: st.db.settings().map_err(err)?,
    })
    .map_err(err)?;
    let _ = app.opener().reveal_item_in_dir(&path);
    Ok(path.display().to_string())
}

/// Open the log folder (LOCALAPPDATA/VoidTrace/logs) in Explorer.
#[tauri::command]
pub fn open_log_folder(app: AppHandle) -> R<()> {
    use tauri_plugin_opener::OpenerExt;
    let dir = crate::diag::log_dir();
    std::fs::create_dir_all(&dir).map_err(err)?;
    app.opener().open_path(dir.display().to_string(), None::<&str>).map_err(err)
}

/// Rebind the hotkey of an overlay action ("edit" or "hud") and remember it.
#[tauri::command]
pub fn set_overlay_hotkey(app: AppHandle, st: State<AppState>, action: String, key: String) -> R<()> {
    let (_, setting, _) = crate::overlay::HOTKEYS.iter().find(|(a, _, _)| *a == action).ok_or("unknown hotkey action")?;
    crate::overlay::register_hotkey(&app, &action, &key).map_err(err)?;
    st.db.set(setting, &key).map_err(err)
}

/// Current hotkeys by action.
#[tauri::command]
pub fn overlay_hotkeys(st: State<AppState>) -> std::collections::HashMap<String, String> {
    st.hotkeys.read().clone()
}

/// Whether the widgets are hidden by the HUD hotkey.
#[tauri::command]
pub fn hud_hidden(st: State<AppState>) -> bool {
    *st.hud_hidden.read()
}

#[tauri::command]
pub fn set_hud_hidden(app: AppHandle, hidden: bool) {
    crate::overlay::set_hud_hidden(&app, hidden);
}

/// Start with Windows (hidden, `--background`) and open when Warframe starts.
#[tauri::command]
pub fn set_launch_with_game(app: AppHandle, st: State<AppState>, on: bool) -> R<()> {
    use tauri_plugin_autostart::ManagerExt;
    let al = app.autolaunch();
    if on {
        al.enable().map_err(err)?;
    } else if al.is_enabled().unwrap_or(false) {
        al.disable().map_err(err)?;
    }
    st.db.set("launch_with_game", if on { "1" } else { "0" }).map_err(err)
}

/// Catalogue entry for a slug (the inventory ledger stores slugs, orders need ids).
#[tauri::command]
pub fn market_item(st: State<AppState>, slug: String) -> Option<MarketItem> {
    st.market.items().into_iter().find(|i| i.slug == slug)
}

#[tauri::command]
pub async fn market_price(st: State<'_, AppState>, slug: String) -> R<PriceSummary> {
    st.market.price(&slug).await.map_err(err)
}

#[tauri::command]
pub async fn market_orders(st: State<'_, AppState>, slug: String) -> R<Vec<Order>> {
    st.market.orders(&slug).await.map_err(err)
}

#[tauri::command]
pub async fn wfm_login(app: AppHandle, st: State<'_, AppState>, email: String, password: String) -> R<String> {
    let name = st.market.login(&email, &password).await.map_err(err)?;
    if let Some(t) = st.market.token() {
        st.db.set("wfm_token", &t).map_err(err)?;
    }
    st.db.set("wfm_name", &name).map_err(err)?;
    crate::connect_ws(&app);
    Ok(name)
}

#[tauri::command]
pub async fn wfm_logout(app: AppHandle, st: State<'_, AppState>) -> R<()> {
    let sock = st.ws.write().take();
    if let Some(s) = sock {
        s.close().await;
    }
    st.market.set_token(None);
    st.db.set("wfm_token", "").map_err(err)?;
    crate::connect_ws(&app); // price alerts keep working signed out
    Ok(())
}

#[tauri::command]
pub fn wfm_status(st: State<AppState>) -> Option<wfc_core::ws::RichStatus> {
    st.wfm_status.read().clone()
}

#[tauri::command]
pub async fn wfm_set_status(
    st: State<'_, AppState>,
    status: String,
    activity: Option<String>,
    details: Option<String>,
) -> R<()> {
    let sock = st.ws.read().clone().ok_or("socket not connected (log in first)")?;
    let activity = activity.map(|kind| wfc_core::ws::Activity {
        kind,
        details,
        started_at: chrono::Utc::now().timestamp().into(),
    });
    sock.set_status(&status, None, activity).await.map_err(err)
}

#[tauri::command]
pub async fn wfm_chats(st: State<'_, AppState>) -> R<Vec<wfc_core::market::Chat>> {
    st.market.chats().await.map_err(err)
}

#[tauri::command]
pub async fn wfm_chat(st: State<'_, AppState>, id: String) -> R<Vec<wfc_core::market::ChatMessage>> {
    st.market.chat_messages(&id).await.map_err(err)
}

#[tauri::command]
pub async fn wfm_chat_send(st: State<'_, AppState>, id: String, message: String) -> R<()> {
    st.market.chat_send(&id, &message).await.map_err(err)
}

#[tauri::command]
pub fn open_url(app: AppHandle, url: String) -> R<()> {
    use tauri_plugin_opener::OpenerExt;
    if !url.starts_with("https://warframe.market/") && !url.starts_with("https://github.com/vxnsin/voidtrace/") {
        return Err("only warframe.market and VoidTrace links".into());
    }
    app.opener().open_url(url, None::<&str>).map_err(err)
}

/// Closed orders from the logged-in user's public warframe.market statistics.
#[tauri::command]
pub async fn wfm_closed_orders(st: State<'_, AppState>) -> R<Vec<wfc_core::market::ClosedOrder>> {
    let name = st.db.get("wfm_name").filter(|n| !n.is_empty()).ok_or("log in to warframe.market first")?;
    st.market.closed_orders(&name).await.map_err(err)
}

#[tauri::command]
pub async fn wfm_my_orders(st: State<'_, AppState>) -> R<Vec<Order>> {
    st.market.my_orders().await.map_err(err)
}

#[tauri::command]
pub async fn wfm_create_order(
    st: State<'_, AppState>,
    item_id: String,
    sell: bool,
    platinum: u32,
    quantity: u32,
    rank: Option<u8>,
) -> R<Order> {
    st.market.create_order(&item_id, sell, platinum, quantity, rank).await.map_err(err)
}

#[tauri::command]
pub async fn wfm_update_order(st: State<'_, AppState>, id: String, platinum: u32, quantity: u32, visible: bool) -> R<Order> {
    st.market.update_order(&id, platinum, quantity, visible).await.map_err(err)
}

#[tauri::command]
pub async fn wfm_delete_order(st: State<'_, AppState>, id: String) -> R<()> {
    st.market.delete_order(&id).await.map_err(err)
}

#[tauri::command]
pub fn inventory(st: State<AppState>) -> R<Vec<InvRow>> {
    st.db.inventory().map_err(err)
}
#[tauri::command]
pub fn inventory_set(st: State<AppState>, slug: String, name: String, quantity: i32) -> R<()> {
    st.db.inventory_set(&slug, &name, quantity).map_err(err)
}
#[tauri::command]
pub fn inventory_add(st: State<AppState>, slug: String, name: String, delta: i32) -> R<i32> {
    st.db.inventory_add(&slug, &name, delta).map_err(err)
}

#[tauri::command]
pub fn grade_riven(st: State<AppState>, weapon: String, stats: Vec<StatInput>) -> R<RivenGrade> {
    let ex = export(&st)?;
    Ok(wfc_core::riven::grade(&ex, &weapon, &stats))
}

#[tauri::command]
pub fn trades(st: State<AppState>) -> R<Vec<Trade>> {
    st.db.trades().map_err(err)
}

#[derive(Deserialize)]
pub struct NewTrade {
    pub item: String,
    pub quantity: u32,
    pub platinum: i32,
    pub direction: String,
    pub partner: String,
}

#[tauri::command]
pub fn add_trade(st: State<AppState>, t: NewTrade) -> R<i64> {
    st.db
        .add_trade(&Trade {
            id: 0,
            ts: chrono::Utc::now().timestamp(),
            item: t.item,
            quantity: t.quantity,
            platinum: t.platinum,
            direction: t.direction,
            partner: t.partner,
        })
        .map_err(err)
}
#[derive(Deserialize)]
pub struct TradeDone {
    /// warframe.market order to close; its quantity drops by `quantity`.
    pub order_id: Option<String>,
    /// Inventory ledger row to adjust (sell: minus, buy: plus).
    pub slug: Option<String>,
    pub item: String,
    pub quantity: u32,
    /// Total for the whole trade, like the manual entries in the Stats tab.
    pub platinum: i32,
    pub direction: String,
    pub partner: String,
}

/// A finished in-game trade: close the market order first (if any), so a
/// failed close leaves nothing half-recorded, then log it and move the ledger.
#[tauri::command]
pub async fn record_trade(app: AppHandle, st: State<'_, AppState>, t: TradeDone) -> R<i64> {
    if let Some(id) = &t.order_id {
        st.market.close_order(id, t.quantity).await.map_err(err)?;
    }
    let verb = if t.direction == "sell" { "Sold" } else { "Bought" };
    let qty = if t.quantity > 1 { format!(" ×{}", t.quantity) } else { String::new() };
    let mut n = crate::notify::Notice::new(
        "trade",
        format!("{verb} {}{qty} for {}p", t.item, t.platinum),
        if t.partner.is_empty() { String::new() } else { format!("{} {}", if t.direction == "sell" { "to" } else { "from" }, t.partner) },
    );
    if let Some(slug) = &t.slug {
        n = n.item(&st, slug);
    }
    crate::notify::send(&app, n);
    let id = st
        .db
        .add_trade(&Trade {
            id: 0,
            ts: chrono::Utc::now().timestamp(),
            item: t.item.clone(),
            quantity: t.quantity,
            platinum: t.platinum,
            direction: t.direction.clone(),
            partner: t.partner,
        })
        .map_err(err)?;
    if let Some(slug) = &t.slug {
        let delta = if t.direction == "sell" { -(t.quantity as i32) } else { t.quantity as i32 };
        let left = st.db.inventory_add(slug, &t.item, delta).map_err(err)?;
        if left < 0 {
            st.db.inventory_set(slug, &t.item, 0).map_err(err)?;
        }
    }
    Ok(id)
}

#[tauri::command]
pub fn delete_trade(st: State<AppState>, id: i64) -> R<()> {
    st.db.delete_trade(id).map_err(err)
}

#[tauri::command]
pub fn datapoints(st: State<AppState>, limit: usize) -> R<Vec<Datapoint>> {
    st.db.datapoints(limit).map_err(err)
}
#[tauri::command]
pub fn add_datapoint(st: State<AppState>, platinum: i32, ducats: i32, credits: i64) -> R<()> {
    st.db
        .add_datapoint(&Datapoint { ts: chrono::Utc::now().timestamp(), platinum, ducats, credits })
        .map_err(err)
}
#[tauri::command]
pub fn relic_runs(st: State<AppState>, limit: usize) -> R<Vec<RelicRun>> {
    st.db.relic_runs(limit).map_err(err)
}

#[tauri::command]
pub async fn ocr_now(app: AppHandle) -> R<()> {
    crate::overlay::run_ocr(&app).await;
    Ok(())
}

#[tauri::command]
pub fn overlay_toggle(app: AppHandle) -> R<bool> {
    let w = app.get_webview_window("overlay").ok_or("no overlay")?;
    let vis = w.is_visible().map_err(err)?;
    if vis { crate::overlay::hide(&app) } else { crate::overlay::show(&app) }
    Ok(!vis)
}

#[tauri::command]
pub fn notices() -> Vec<crate::notify::Notice> {
    crate::notify::recent()
}
#[tauri::command]
pub fn notify_kinds() -> Vec<&'static str> {
    crate::notify::KINDS.to_vec()
}
/// Show a sample of one kind in game (and on the Discord hooks that take it).
#[tauri::command]
pub fn notify_test(app: AppHandle, kind: String) {
    use crate::notify::Notice;
    let n = match kind.as_str() {
        "cycle" => Notice::new("cycle", "Cetus: night in 5 min", "day until 21:40"),
        "fissure" => Notice::new("fissure", "Omnia Capture · Steel Path", "Hepit (Void) · Corrupted · 54 min left"),
        "baro" => Notice::new("baro", "Baro Ki'Teer is at Larunda Relay (Mercury)", "Worth a look: Primed Chamber 210p"),
        "undercut" => Notice::new("undercut", "Undercut: Ash Prime Set", "someone sells at 55p, you're at 60p"),
        "price" => Notice::new("price", "Ash Prime Set for 50p", "someone just listed it (alert at 52p or less)"),
        "trade" => Notice::new("trade", "Sold Ash Prime Set for 60p", "to someone"),
        _ => Notice::new(&kind, "Test", "This is what a notice looks like"),
    };
    crate::notify::send(&app, n);
}
#[tauri::command]
pub async fn discord_test(st: State<'_, AppState>, url: String) -> R<()> {
    let n = crate::notify::Notice::new("test", "VoidTrace is connected", "Notifications you picked for this webhook will show up here.");
    crate::notify::discord(&st.http, url.trim(), &n).await.map_err(err)
}

#[tauri::command]
pub fn market_watch() -> crate::watch::Snapshot {
    crate::watch::last()
}
#[tauri::command]
pub async fn market_watch_refresh(app: AppHandle) -> R<crate::watch::Snapshot> {
    crate::watch::check(&app).await.map_err(err)
}
#[tauri::command]
pub fn price_alerts(st: State<AppState>) -> Vec<crate::watch::PriceAlert> {
    crate::watch::alerts(&st)
}
/// Save the alerts and make sure the new-orders feed is on when there are any.
#[tauri::command]
pub async fn set_price_alerts(app: AppHandle, st: State<'_, AppState>, alerts: Vec<crate::watch::PriceAlert>) -> R<()> {
    st.db.set("price_alerts", &serde_json::to_string(&alerts).map_err(err)?).map_err(err)?;
    let sock = st.ws.read().clone();
    match sock {
        Some(s) if !alerts.is_empty() => s.subscribe(wfc_core::ws::new_orders_sub()).await.map_err(err)?,
        None if !alerts.is_empty() => crate::connect_ws(&app),
        _ => {}
    }
    Ok(())
}
