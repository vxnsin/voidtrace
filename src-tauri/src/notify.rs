//! Notifications: everything worth knowing goes through `send`, which shows it
//! in the notification widget (event "notice") and posts it to the Discord
//! webhooks that asked for that kind. The world state checks (cycles about to
//! flip, matching fissures, Baro) live here too; market checks are in watch.rs.

use crate::AppState;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::LazyLock;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

/// Kinds, in the order the settings list them.
pub const KINDS: &[&str] = &["cycle", "fissure", "baro", "undercut", "price", "trade", "riven"];

#[derive(Debug, Clone, Serialize, Default)]
pub struct Notice {
    pub kind: String,
    pub title: String,
    pub body: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    /// Full picture URL (warframe.market static assets).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    pub ts: i64,
    /// Play the sound for this one (from the settings, filled in by `send`).
    pub sound: bool,
}

impl Notice {
    pub fn new(kind: &str, title: impl Into<String>, body: impl Into<String>) -> Self {
        Notice { kind: kind.into(), title: title.into(), body: body.into(), ts: chrono::Utc::now().timestamp(), ..Default::default() }
    }
    /// Link and picture of a warframe.market item.
    pub fn item(mut self, st: &AppState, slug: &str) -> Self {
        if let Some(i) = st.market.items().into_iter().find(|i| i.slug == slug) {
            if !i.icon.is_empty() {
                self.icon = Some(format!("https://warframe.market/static/assets/{}", i.icon));
            }
        }
        self.url = Some(format!("https://warframe.market/items/{slug}"));
        self.slug = Some(slug.to_string());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FissureRule {
    pub tiers: Vec<String>,
    pub missions: Vec<String>,
    /// "all", "only" or "hide"
    #[serde(rename = "steelPath")]
    pub steel_path: String,
    pub storms: bool,
}

/// Same defaults as the settings page shows: every fissure, storms included.
impl Default for FissureRule {
    fn default() -> Self {
        Self { tiers: vec![], missions: vec![], steel_path: "all".into(), storms: true }
    }
}

/// Setting `notify`; written by the Widgets tab.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Kinds not shown in game (Discord has its own list per webhook).
    pub off: Vec<String>,
    /// Kinds that play a sound.
    pub sound: Vec<String>,
    /// Minutes before a cycle flips.
    pub cycle_lead: u32,
    /// Which cycles (`cetus`, `vallis`, …); empty = all.
    pub cycles: Vec<String>,
    /// Fissures to announce; no tier and no mission type = none.
    pub fissure: FissureRule,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            off: vec![],
            sound: vec![],
            cycle_lead: 5,
            cycles: vec![],
            fissure: FissureRule { steel_path: "all".into(), storms: true, ..Default::default() },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hook {
    pub name: String,
    pub url: String,
    pub kinds: Vec<String>,
}

pub fn config(st: &AppState) -> Config {
    st.db.get("notify").and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}
pub fn hooks(st: &AppState) -> Vec<Hook> {
    st.db.get("discord_hooks").and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

#[derive(Default)]
struct Memory {
    recent: VecDeque<Notice>,
    /// Discord posts per webhook in the last minute.
    posts: HashMap<String, VecDeque<Instant>>,
    cycles: HashSet<String>,
    fissures: Option<HashSet<String>>,
}
static MEM: LazyLock<Mutex<Memory>> = LazyLock::new(Default::default);

/// The last notices, newest first (for the app).
pub fn recent() -> Vec<Notice> {
    MEM.lock().recent.iter().cloned().collect()
}

pub fn send(app: &AppHandle, mut n: Notice) {
    let st = app.state::<AppState>();
    let cfg = config(&st);
    tracing::info!("notice [{}] {} — {}", n.kind, n.title, n.body);
    n.sound = cfg.sound.contains(&n.kind);
    {
        let mut m = MEM.lock();
        m.recent.push_front(n.clone());
        m.recent.truncate(50);
    }
    if !cfg.off.contains(&n.kind) {
        let _ = app.emit("notice", &n);
    }
    for h in hooks(&st).into_iter().filter(|h| h.kinds.contains(&n.kind)) {
        if !allow_post(&h.url) {
            tracing::warn!("discord {}: rate limit, dropped \"{}\"", h.name, n.title);
            continue;
        }
        let http = st.http.clone();
        let n = n.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(e) = discord(&http, &h.url, &n).await {
                tracing::warn!("discord {}: {e}", h.name);
            }
        });
    }
}

/// At most 6 posts per webhook and minute, so a burst of fissures doesn't spam
/// the channel (Discord itself allows ~30).
fn allow_post(url: &str) -> bool {
    let mut m = MEM.lock();
    let q = m.posts.entry(url.to_string()).or_default();
    while q.front().is_some_and(|t| t.elapsed() > Duration::from_secs(60)) {
        q.pop_front();
    }
    if q.len() >= 6 {
        return false;
    }
    q.push_back(Instant::now());
    true
}

pub fn valid_hook(url: &str) -> bool {
    [
        "https://discord.com/api/webhooks/",
        "https://discordapp.com/api/webhooks/",
        "https://ptb.discord.com/api/webhooks/",
        "https://canary.discord.com/api/webhooks/",
    ]
    .iter()
    .any(|p| url.starts_with(p))
}

pub async fn discord(http: &reqwest::Client, url: &str, n: &Notice) -> anyhow::Result<()> {
    anyhow::ensure!(valid_hook(url), "not a Discord webhook URL");
    let mut embed = json!({
        "title": n.title,
        "description": n.body,
        "color": 0xc8102e,
        "timestamp": chrono::DateTime::from_timestamp(n.ts, 0).unwrap_or_default().to_rfc3339(),
        "footer": { "text": format!("VoidTrace · {}", n.kind) },
    });
    if let Some(u) = &n.url {
        embed["url"] = u.as_str().into();
    }
    if let Some(i) = &n.icon {
        embed["thumbnail"] = json!({ "url": i });
    }
    // The webhook URL is the secret; keep it out of error text and logs.
    let r = http
        .post(url)
        .json(&json!({ "username": "VoidTrace", "embeds": [embed] }))
        .send()
        .await
        .map_err(|e| e.without_url())?;
    if !r.status().is_success() {
        let status = r.status();
        anyhow::bail!("{status} {}", r.text().await.unwrap_or_default().chars().take(200).collect::<String>());
    }
    Ok(())
}

const CYCLES: &[(&str, &str, &str)] = &[
    ("cetus", "cetusCycle", "Cetus"),
    ("vallis", "vallisCycle", "Orb Vallis"),
    ("cambion", "cambionCycle", "Cambion Drift"),
    ("zariman", "zarimanCycle", "Zariman"),
    ("duviri", "duviriCycle", "Duviri"),
    ("earth", "earthCycle", "Earth"),
];

fn next_state(state: &str) -> &'static str {
    match state {
        "day" => "night",
        "night" => "day",
        "warm" => "cold",
        "cold" => "warm",
        "fass" => "vome",
        "vome" => "fass",
        "grineer" => "corpus",
        "corpus" => "grineer",
        "joy" => "anger",
        "anger" => "envy",
        "envy" => "sorrow",
        "sorrow" => "fear",
        "fear" => "joy",
        _ => "",
    }
}

fn secs_until(iso: &str) -> Option<i64> {
    let t = chrono::DateTime::parse_from_rfc3339(iso).ok()?;
    Some(t.timestamp() - chrono::Utc::now().timestamp())
}

fn local_time(iso: &str, fmt: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(iso)
        .map(|t| t.with_timezone(&chrono::Local).format(fmt).to_string())
        .unwrap_or_default()
}

fn fissure_matches(f: &Value, r: &FissureRule) -> bool {
    if r.tiers.is_empty() && r.missions.is_empty() {
        return false;
    }
    let s = |k: &str| f[k].as_str().unwrap_or("");
    let hard = f["isHard"].as_bool().unwrap_or(false);
    let storm = f["isStorm"].as_bool().unwrap_or(false);
    (r.tiers.is_empty() || r.tiers.iter().any(|t| t == s("tier")))
        && (r.missions.is_empty() || r.missions.iter().any(|m| m == s("missionType")))
        && match r.steel_path.as_str() {
            "only" => hard,
            "hide" => !hard,
            _ => true,
        }
        && (r.storms || !storm)
}

/// Called with every new world state (once a minute).
pub fn on_worldstate(app: &AppHandle, ws: &Value) {
    let st = app.state::<AppState>();
    let cfg = config(&st);
    let mut out = vec![];
    {
        let mut m = MEM.lock();
        for (id, key, place) in CYCLES {
            if !cfg.cycles.is_empty() && !cfg.cycles.iter().any(|c| c == id) {
                continue;
            }
            let c = &ws[*key];
            let (Some(exp), Some(state)) = (c["expiry"].as_str(), c["state"].as_str()) else { continue };
            let Some(left) = secs_until(exp) else { continue };
            let next = next_state(state);
            if next.is_empty() || left <= 0 || left > cfg.cycle_lead as i64 * 60 {
                continue;
            }
            if m.cycles.insert(format!("{id}:{exp}")) {
                let mins = (left + 59) / 60;
                out.push(Notice::new("cycle", format!("{place}: {next} in {mins} min"), format!("{state} until {}", local_time(exp, "%H:%M"))));
            }
        }

        let list = ws["fissures"].as_array().cloned().unwrap_or_default();
        let ids: HashSet<String> = list.iter().filter_map(|f| f["id"].as_str().map(String::from)).collect();
        // First world state after start: everything is "old", nothing to announce.
        let seen = m.fissures.get_or_insert_with(|| ids.clone());
        for f in &list {
            let Some(id) = f["id"].as_str() else { continue };
            if seen.contains(id) || !fissure_matches(f, &cfg.fissure) {
                continue;
            }
            let s = |k: &str| f[k].as_str().unwrap_or("").to_string();
            let mut what = format!("{} {}", s("tier"), s("missionType"));
            if f["isHard"].as_bool() == Some(true) {
                what += " · Steel Path";
            }
            if f["isStorm"].as_bool() == Some(true) {
                what += " · Void Storm";
            }
            let left = secs_until(&s("expiry")).unwrap_or(0).max(0) / 60;
            out.push(Notice::new("fissure", what, format!("{} · {} · {left} min left", s("node"), s("enemy"))));
        }
        *seen = ids;
    }
    for n in out {
        send(app, n);
    }
    baro(app, ws);
}

/// Once per visit (remembered across restarts): where he is and the best of
/// his offer by warframe.market price.
fn baro(app: &AppHandle, ws: &Value) {
    let st = app.state::<AppState>();
    let b = &ws["voidTrader"];
    let (Some(act), Some(exp)) = (b["activation"].as_str(), b["expiry"].as_str()) else { return };
    if secs_until(act).unwrap_or(1) > 0 || secs_until(exp).unwrap_or(0) <= 0 {
        return;
    }
    if st.db.get("notify_baro_seen").as_deref() == Some(act) {
        return;
    }
    let _ = st.db.set("notify_baro_seen", act);
    let location = b["location"].as_str().unwrap_or("a relay").to_string();
    let until = local_time(exp, "%a %H:%M");
    let names: Vec<String> = b["inventory"]
        .as_array()
        .map(|a| a.iter().filter_map(|i| i["item"].as_str().map(String::from)).collect())
        .unwrap_or_default();
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let st = app.state::<AppState>();
        let mut priced = vec![];
        for name in &names {
            let Some(item) = st.market.find_item(name) else { continue };
            if let Ok(p) = st.market.price(&item.slug).await {
                if let Some(s) = p.sell {
                    priced.push((s, item.name));
                }
            }
            tokio::time::sleep(Duration::from_millis(400)).await; // 3 req/s rule
            if priced.len() >= 12 {
                break;
            }
        }
        priced.sort_by(|a, b| b.0.cmp(&a.0));
        let body = if priced.is_empty() {
            format!("{} items, until {until}", names.len())
        } else {
            let best: Vec<String> = priced.iter().take(3).map(|(p, n)| format!("{n} {p}p")).collect();
            format!("Until {until}. Worth a look: {}", best.join(", "))
        };
        send(&app, Notice::new("baro", format!("Baro Ki'Teer is at {location}"), body));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fissure_rule() {
        let f = json!({ "tier": "Omnia", "missionType": "Capture", "isHard": true, "isStorm": false });
        let mut r = Config::default().fissure;
        assert!(!fissure_matches(&f, &r), "empty rule announces nothing");
        r.tiers = vec!["Omnia".into()];
        assert!(fissure_matches(&f, &r));
        r.steel_path = "hide".into();
        assert!(!fissure_matches(&f, &r));
        r.steel_path = "all".into();
        r.missions = vec!["Survival".into()];
        assert!(!fissure_matches(&f, &r));
    }

    #[test]
    fn cycle_order() {
        assert_eq!(next_state("night"), "day");
        assert_eq!(next_state("fear"), "joy");
        assert_eq!(next_state("unknown"), "");
    }
}
