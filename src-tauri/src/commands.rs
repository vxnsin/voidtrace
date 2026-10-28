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

/// Write everything to `path`: one JSON file, or CSV files named after it.
/// Returns the files written.
#[tauri::command]
pub fn export_data(app: AppHandle, st: State<AppState>, path: String, format: String) -> R<Vec<String>> {
    let b = st.db.backup(&app.package_info().version.to_string()).map_err(err)?;
    let p = std::path::PathBuf::from(&path);
    let files = if format == "csv" {
        b.write_csv(&p).map_err(err)?
    } else {
        std::fs::write(&p, serde_json::to_vec_pretty(&b).map_err(err)?).map_err(err)?;
        vec![p]
    };
    tracing::info!("exported {} trades to {path}", b.trades.len());
    Ok(files.iter().map(|f| f.display().to_string()).collect())
}

/// Merge a JSON export (from this or another PC) into the local data.
#[tauri::command]
pub fn import_data(st: State<AppState>, path: String) -> R<wfc_core::db::Imported> {
    let text = std::fs::read_to_string(&path).map_err(err)?;
    let b: wfc_core::db::Backup = serde_json::from_str(&text).map_err(|e| format!("not a VoidTrace export: {e}"))?;
    let n = st.db.restore(&b).map_err(err)?;
    tracing::info!("imported from {path}: {n:?}");
    Ok(n)
}

#[tauri::command]
pub async fn market_stats(st: State<'_, AppState>, slug: String) -> R<wfc_core::market::ItemStats> {
    st.market.item_statistics(&slug).await.map_err(err)
}

/// Relics from the last inventory sync: name -> count per refinement.
fn owned_relics(st: &AppState) -> std::collections::HashMap<String, [i64; 4]> {
    st.db.get("owned_relics").and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

#[derive(Serialize)]
pub struct RelicRow {
    pub name: String,
    pub owned: [i64; 4],
    pub vaulted: bool,
    pub plan: RelicPlan,
    /// Drops without a known price yet.
    pub missing: usize,
}

#[derive(Serialize)]
pub struct RelicOverview {
    pub rows: Vec<RelicRow>,
    pub synced: bool,
    /// Prices still being fetched in the background.
    pub pending: usize,
}

static WARMING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// Slugs whose price fetch failed, with when. They are skipped for half an
/// hour, otherwise the UI's "ask again" after each pass would retry them forever.
static PRICE_FAILED: std::sync::LazyLock<parking_lot::Mutex<std::collections::HashMap<String, std::time::Instant>>> =
    std::sync::LazyLock::new(Default::default);

fn drop_failed(slugs: &mut Vec<String>) {
    let failed = PRICE_FAILED.lock();
    slugs.retain(|s| failed.get(s).is_none_or(|t| t.elapsed().as_secs() >= 1800));
}

/// Fetch missing prices one by one at the market's pace and tell the UI via
/// `event` every ten and at the end. `flag` keeps it to one pass at a time.
fn warm_prices(app: &AppHandle, slugs: Vec<String>, event: &'static str, flag: &'static std::sync::atomic::AtomicBool) {
    use std::sync::atomic::Ordering::SeqCst;
    if slugs.is_empty() || flag.swap(true, SeqCst) {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let st = app.state::<AppState>();
        for (i, slug) in slugs.iter().enumerate() {
            if let Err(e) = st.market.price(slug).await {
                tracing::debug!("price {slug}: {e}");
                PRICE_FAILED.lock().insert(slug.clone(), std::time::Instant::now());
            }
            if i % 10 == 9 || i + 1 == slugs.len() {
                use tauri::Emitter;
                let _ = app.emit(event, (i + 1, slugs.len()));
            }
            tokio::time::sleep(std::time::Duration::from_millis(400)).await;
        }
        flag.store(false, SeqCst);
    });
}

/// Every relic (or only owned ones) with expected values from prices known so
/// far. Missing prices are fetched in the background at the market's pace;
/// "relic-prices" events tell the UI to ask again.
#[tauri::command]
pub fn relic_overview(app: AppHandle, st: State<AppState>, only_owned: bool) -> R<RelicOverview> {
    let ex = export(&st)?;
    let owned = owned_relics(&st);
    let index: std::collections::HashMap<String, MarketItem> =
        st.market.items().into_iter().map(|i| (wfc_core::norm(&i.name), i)).collect();
    let lookup = |n: &str| index.get(&wfc_core::norm(n)).cloned();
    let mut rows = vec![];
    let mut missing: Vec<String> = vec![];
    let mut seen = std::collections::HashSet::new();
    for r in &ex.relics {
        // The export lists each relic once per refinement; one row per name.
        if !seen.insert(r.name.clone()) {
            continue;
        }
        let have = owned.get(&r.name).copied().unwrap_or_default();
        if only_owned && have.iter().sum::<i64>() == 0 {
            continue;
        }
        let (plan, miss) = wfc_core::relic::plan_cached(&ex, &st.market, &lookup, r);
        let vaulted = lookup(&format!("{} Relic", r.name)).map(|i| i.vaulted).unwrap_or(false);
        rows.push(RelicRow { name: r.name.clone(), owned: have, vaulted, missing: miss.len(), plan });
        missing.extend(miss);
    }
    missing.sort();
    missing.dedup();
    drop_failed(&mut missing);
    let pending = missing.len();
    warm_prices(&app, missing, "relic-prices", &WARMING);
    Ok(RelicOverview { rows, synced: st.db.get("owned_relics").is_some(), pending })
}

// ---- Foundry, sets, mastery routes (#12, #13) ----

use wfc_core::foundry::{self, Build, Owned};

fn owned_items(st: &AppState) -> Owned {
    st.db.get("owned_items").and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

#[derive(Serialize)]
pub struct FoundryRow {
    #[serde(flatten)]
    pub build: Build,
    pub mastered: bool,
    pub xp: i64,
}

#[derive(Serialize)]
pub struct FoundryOverview {
    pub rows: Vec<FoundryRow>,
    /// The inventory sync has run (owned counts are real).
    pub synced: bool,
    pub mastery_rank: u32,
}

#[tauri::command]
pub fn foundry_overview(st: State<AppState>) -> R<FoundryOverview> {
    let ex = export(&st)?;
    let items = st.market.items();
    let idx = foundry::Index::new(&ex, &items);
    let owned = owned_items(&st);
    let p = st.db.get("profile_cache").and_then(|s| serde_json::from_str(&s).ok());
    let m = wfc_core::mastery::summary(&ex, p.as_ref());
    let xp: std::collections::HashMap<&str, (bool, i64)> = m.items.iter().map(|i| (i.unique_name.as_str(), (i.mastered, i.xp))).collect();
    let rows = ex
        .all_masterable()
        .map(|it| {
            let (mastered, xp) = xp.get(it.unique_name.as_str()).copied().unwrap_or((false, 0));
            FoundryRow { build: foundry::build(&ex, &idx, &owned, it), mastered, xp }
        })
        .collect();
    Ok(FoundryOverview { rows, synced: !owned.is_empty(), mastery_rank: m.rank })
}

#[derive(Serialize)]
pub struct PartSource {
    pub name: String,
    pub slug: Option<String>,
    pub plat: Option<u32>,
    pub relics: Vec<RelicHit>,
}

/// Where to get each part of one item: relics that drop it and the market price.
#[tauri::command]
pub async fn foundry_sources(st: State<'_, AppState>, unique_name: String) -> R<Vec<PartSource>> {
    let ex = export(&st)?;
    let items = st.market.items();
    let it = ex.all_masterable().find(|i| i.unique_name == unique_name).ok_or("unknown item")?.clone();
    let wanted: Vec<(String, Option<String>)> = {
        let idx = foundry::Index::new(&ex, &items);
        let b = foundry::build(&ex, &idx, &owned_items(&st), &it);
        let mut w = vec![];
        if let Some(bp) = &b.blueprint {
            w.push((ex.item_name(bp), b.bp_slug.clone()));
        }
        for p in b.parts.iter().filter(|p| !foundry::is_resource(&p.path)) {
            w.push((p.blueprint.as_deref().map(|x| ex.item_name(x)).unwrap_or(p.name.clone()), p.slug.clone()));
        }
        w
    };
    let mut out = vec![];
    for (name, slug) in wanted {
        let plat = match &slug {
            Some(s) => st.market.price(s).await.ok().and_then(|p| p.sell),
            None => None,
        };
        let relics = wfc_core::relic::relics_for(&ex, &name)
            .into_iter()
            .take(6)
            .map(|(relic, rarity, chance)| {
                let vaulted = st.market.find_item_exact(&format!("{relic} Relic")).map(|i| i.vaulted).unwrap_or(false);
                RelicHit { relic, rarity, chance, vaulted }
            })
            .collect();
        out.push(PartSource { name, slug, plat, relics });
    }
    Ok(out)
}

#[tauri::command]
pub fn inventory_sets(st: State<AppState>) -> R<Vec<foundry::SetInfo>> {
    let ex = export(&st)?;
    let items = st.market.items();
    let idx = foundry::Index::new(&ex, &items);
    let have: std::collections::HashMap<String, i64> =
        st.db.inventory().map_err(err)?.into_iter().map(|r| (r.slug, r.quantity as i64)).collect();
    Ok(foundry::sets(&idx, &items, &have))
}

#[derive(Serialize)]
pub struct BuyPart {
    pub name: String,
    pub slug: String,
    pub plat: Option<u32>,
}

#[derive(Serialize)]
pub struct PlatRoute {
    pub unique_name: String,
    pub name: String,
    pub category: String,
    pub image: Option<String>,
    pub missing: Vec<BuyPart>,
    /// Sum of the cheapest sellers, once every part has a price.
    pub total: Option<u32>,
}

#[derive(Serialize)]
pub struct Intrinsic {
    pub name: String,
    pub rank: i64,
}

#[derive(Serialize)]
pub struct MasteryRoutes {
    pub relics: Vec<foundry::RelicRoute>,
    pub plat: Vec<PlatRoute>,
    pub intrinsics: Vec<Intrinsic>,
    pub synced: bool,
    pub pending: usize,
}

static MASTERY_WARMING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Ways to the next mastery: from relics I own, or by buying parts (cheapest
/// first; prices load in the background), plus missing intrinsics.
#[tauri::command]
pub fn mastery_routes(app: AppHandle, st: State<AppState>) -> R<MasteryRoutes> {
    let ex = export(&st)?;
    let items = st.market.items();
    let idx = foundry::Index::new(&ex, &items);
    let owned = owned_items(&st);
    let p = st.db.get("profile_cache").and_then(|s| serde_json::from_str(&s).ok());
    let m = wfc_core::mastery::summary(&ex, p.as_ref());
    let done: std::collections::HashSet<&str> = m.items.iter().filter(|i| i.mastered).map(|i| i.unique_name.as_str()).collect();
    let started: std::collections::HashSet<&str> = m.items.iter().filter(|i| i.xp > 0).map(|i| i.unique_name.as_str()).collect();
    let todo: Vec<&wfc_core::export::ExportItem> = ex.all_masterable().filter(|i| !done.contains(i.unique_name.as_str())).collect();

    let relics = foundry::relic_routes(&ex, &idx, &owned, &owned_relics(&st), &todo).into_iter().take(60).collect();

    let mut missing_prices = vec![];
    let mut plat: Vec<PlatRoute> = todo
        .iter()
        .filter(|i| !started.contains(i.unique_name.as_str()))
        .filter_map(|it| {
            let b = foundry::build(&ex, &idx, &owned, it);
            let mut buy = vec![];
            if b.blueprint.is_some() && b.bp_have == 0 {
                buy.push((ex.item_name(b.blueprint.as_deref().unwrap()), b.bp_slug.clone()?));
            }
            for p in b.parts.iter().filter(|p| !foundry::is_resource(&p.path)) {
                if p.have + p.bp_have < p.need {
                    // Not tradable (resources, non-prime parts): no plat route.
                    let slug = p.slug.clone()?;
                    for _ in 0..(p.need - p.have - p.bp_have) {
                        buy.push((p.blueprint.as_deref().map(|x| ex.item_name(x)).unwrap_or(p.name.clone()), slug.clone()));
                    }
                }
            }
            if buy.is_empty() {
                return None;
            }
            let missing: Vec<BuyPart> = buy
                .into_iter()
                .map(|(name, slug)| {
                    let cached = st.market.cached_price(&slug);
                    if cached.is_none() {
                        missing_prices.push(slug.clone());
                    }
                    BuyPart { name, plat: cached.and_then(|(_, p)| p.sell), slug }
                })
                .collect();
            let total = missing.iter().map(|m| m.plat).sum::<Option<u32>>();
            Some(PlatRoute { unique_name: b.unique_name, name: b.name, category: b.category, image: b.image, missing, total })
        })
        .collect();
    plat.sort_by_key(|r| r.total.unwrap_or(u32::MAX));
    plat.truncate(120);

    missing_prices.sort();
    missing_prices.dedup();
    drop_failed(&mut missing_prices);
    let pending = missing_prices.len();
    warm_prices(&app, missing_prices, "mastery-prices", &MASTERY_WARMING);

    let skills: serde_json::Value = st.db.get("intrinsics").and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
    let intrinsics = foundry::INTRINSICS
        .iter()
        .map(|(k, name)| Intrinsic { name: name.to_string(), rank: skills[*k].as_i64().unwrap_or(0) })
        .collect();
    Ok(MasteryRoutes { relics, plat, intrinsics, synced: !owned.is_empty(), pending })
}

// ---- Rivens (#11) ----

static RIVEN_WEAPONS: parking_lot::Mutex<Vec<wfc_core::riven::RivenWeapon>> = parking_lot::Mutex::new(Vec::new());
static WEEKLY: parking_lot::Mutex<Option<(std::time::Instant, Vec<wfc_core::riven::WeeklyRiven>)>> = parking_lot::Mutex::new(None);

#[tauri::command]
pub async fn riven_weapons(st: State<'_, AppState>) -> R<Vec<wfc_core::riven::RivenWeapon>> {
    if RIVEN_WEAPONS.lock().is_empty() {
        let list = wfc_core::riven::riven_weapons(&st.http).await.map_err(err)?;
        *RIVEN_WEAPONS.lock() = list;
    }
    Ok(RIVEN_WEAPONS.lock().clone())
}

#[derive(Serialize)]
pub struct RivenSearch {
    pub auctions: Vec<wfc_core::riven::RivenAuction>,
    pub prices: wfc_core::riven::RivenPrices,
    /// DE numbers for the last weeks (unrolled and rerolled).
    pub weekly: Vec<wfc_core::riven::WeeklyRiven>,
}

#[tauri::command]
pub async fn riven_search(st: State<'_, AppState>, weapon: String, name: String) -> R<RivenSearch> {
    let auctions = wfc_core::riven::riven_auctions(&st.http, &weapon).await.map_err(err)?;
    let fresh = WEEKLY.lock().as_ref().is_some_and(|(t, _)| t.elapsed().as_secs() < 6 * 3600);
    if !fresh {
        match wfc_core::riven::weekly_rivens(&st.http).await {
            Ok(w) => *WEEKLY.lock() = Some((std::time::Instant::now(), w)),
            Err(e) => tracing::warn!("weekly rivens: {e}"),
        }
    }
    let weekly = WEEKLY
        .lock()
        .as_ref()
        .map(|(_, w)| w.iter().filter(|r| r.compatibility.as_deref().is_some_and(|c| c.eq_ignore_ascii_case(&name))).cloned().collect())
        .unwrap_or_default();
    let prices = wfc_core::riven::riven_prices(&auctions);
    Ok(RivenSearch { auctions: auctions.into_iter().take(60).collect(), prices, weekly })
}

#[tauri::command]
pub fn riven_snipes(st: State<AppState>) -> Vec<crate::watch::RivenSnipe> {
    crate::watch::snipes(&st)
}
#[tauri::command]
pub fn set_riven_snipes(st: State<AppState>, snipes: Vec<crate::watch::RivenSnipe>) -> R<()> {
    st.db.set("riven_snipes", &serde_json::to_string(&snipes).map_err(err)?).map_err(err)
}
// ---- Phone dashboard (#17) ----

#[tauri::command]
pub async fn dashboard_start(app: AppHandle, st: State<'_, AppState>) -> R<crate::dashboard::DashInfo> {
    let port = st.db.get("dashboard_port").and_then(|p| p.parse().ok()).unwrap_or(47811);
    crate::dashboard::start(&app, port).await.map_err(err)
}
#[tauri::command]
pub fn dashboard_stop(app: AppHandle) -> crate::dashboard::DashInfo {
    crate::dashboard::stop(&app);
    crate::dashboard::info()
}
#[tauri::command]
pub fn dashboard_status() -> crate::dashboard::DashInfo {
    crate::dashboard::info()
}

// ---- Riven overlay (hotkey / cycling screen) ----

#[derive(Serialize, Clone)]
pub struct RivenPriceSummary {
    pub min: Option<u32>,
    pub median: Option<u32>,
    pub count: usize,
}

#[derive(Serialize, Clone)]
pub struct RivenScan {
    pub card: wfc_core::riven_ocr::RivenCard,
    /// warframe.market url_names of the card's positive stats, for the match % per listing.
    pub positives: Vec<String>,
    pub grade: wfc_core::riven::RivenGrade,
    pub similar: Vec<wfc_core::riven::RivenAuction>,
    pub prices: RivenPriceSummary,
    pub suggest: wfc_core::riven_ocr::Suggest,
    /// Listings are still being fetched; the card part is already final.
    pub loading: bool,
}

type AuctionCache = std::collections::HashMap<String, (std::time::Instant, Vec<wfc_core::riven::RivenAuction>)>;
static RIVEN_AUCTIONS: parking_lot::Mutex<Option<AuctionCache>> = parking_lot::Mutex::new(None);
/// Bumped per scan; a running screen watcher stops when it no longer matches.
static RIVEN_GEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
const AUCTION_TTL_SECS: u64 = 300;

/// Load the riven weapon list once (startup), so the first scan doesn't wait for it.
pub async fn prefetch_riven_weapons(app: &AppHandle) {
    let st = app.state::<AppState>();
    if RIVEN_WEAPONS.lock().is_empty() {
        match wfc_core::riven::riven_weapons(&st.http).await {
            Ok(list) => *RIVEN_WEAPONS.lock() = list,
            Err(e) => tracing::warn!("riven weapons: {e}"),
        }
    }
}

fn cached_auctions(slug: &str) -> Option<Vec<wfc_core::riven::RivenAuction>> {
    RIVEN_AUCTIONS
        .lock()
        .as_ref()?
        .get(slug)
        .filter(|(t, _)| t.elapsed().as_secs() < AUCTION_TTL_SECS)
        .map(|(_, a)| a.clone())
}

/// OCR of the card only (cheap); the "Fits in" box is read only when the title
/// can't be matched. `None` = no riven card on screen.
async fn read_card(app: &AppHandle) -> anyhow::Result<Option<wfc_core::riven_ocr::RivenCard>> {
    use wfc_core::ocr::Crop;
    let st = app.state::<AppState>();
    let ex = st.export.read().clone().ok_or_else(|| anyhow::anyhow!("game data not loaded yet"))?;
    if RIVEN_WEAPONS.lock().is_empty() {
        prefetch_riven_weapons(app).await;
    }
    let weapons: Vec<(String, String)> = RIVEN_WEAPONS.lock().iter().map(|w| (w.name.clone(), w.slug.clone())).collect();
    let read = |crop: Crop| async move {
        let (w, h, px) = tokio::task::spawn_blocking(move || wfc_core::ocr::capture_text(crop, 2)).await??;
        tokio::task::spawn_blocking(move || wfc_core::ocr::recognize(w, h, &px)).await?
    };
    let card_lines = read(Crop { x: 0.40, y: 0.58, w: 0.20, h: 0.22 }).await?;
    if let Some(c) = wfc_core::riven_ocr::parse_card(&ex, &weapons, &card_lines, &[]) {
        return Ok(Some(c));
    }
    let fits_lines = read(Crop { x: 0.82, y: 0.64, w: 0.16, h: 0.26 }).await.unwrap_or_default();
    let parsed = wfc_core::riven_ocr::parse_card(&ex, &weapons, &card_lines, &fits_lines);
    if parsed.is_none() {
        tracing::debug!("riven ocr card: {:?} fits in: {:?}", card_lines, fits_lines);
    }
    Ok(parsed)
}

fn build_scan(app: &AppHandle, card: wfc_core::riven_ocr::RivenCard, auctions: Option<&[wfc_core::riven::RivenAuction]>) -> anyhow::Result<RivenScan> {
    let st = app.state::<AppState>();
    let ex = st.export.read().clone().ok_or_else(|| anyhow::anyhow!("game data not loaded yet"))?;
    let grade = wfc_core::riven::grade(&ex, &card.weapon, &card.stats);
    let (_, class) = wfc_core::riven::classify(&ex, &card.weapon);
    let suggest = wfc_core::riven_ocr::suggest(class);
    let list = auctions.unwrap_or(&[]);
    let similar = wfc_core::riven_ocr::similar(&card, list, 2);
    let mut prices: Vec<u32> = list.iter().filter_map(|a| a.buyout.or(Some(a.starting))).filter(|p| *p > 0).collect();
    prices.sort_unstable();
    let summary = RivenPriceSummary {
        min: prices.first().copied(),
        median: (!prices.is_empty()).then(|| prices[prices.len() / 2]),
        count: prices.len(),
    };
    let positives = card.stats.iter().filter(|s| !s.negative).filter_map(|s| wfc_core::riven_ocr::market_attr(&s.stat)).map(str::to_string).collect();
    Ok(RivenScan { loading: auctions.is_none() && card.weapon_slug.is_some(), card, positives, grade, similar, prices: summary, suggest })
}

fn same_roll(a: &wfc_core::riven_ocr::RivenCard, b: &wfc_core::riven_ocr::RivenCard) -> bool {
    a.weapon == b.weapon
        && a.stats.len() == b.stats.len()
        && a.stats.iter().zip(&b.stats).all(|(x, y)| x.stat == y.stat && x.negative == y.negative && (x.value - y.value).abs() < 0.05)
}

/// Show the card part at once, the listings when they arrive.
async fn show_card(app: &AppHandle, gen: u64, card: wfc_core::riven_ocr::RivenCard) {
    use tauri::Emitter;
    let st = app.state::<AppState>();
    let _ = st.db.set("riven_last", &serde_json::to_string(&card).unwrap_or_default());
    let cached = card.weapon_slug.as_deref().and_then(cached_auctions);
    if let Ok(scan) = build_scan(app, card.clone(), cached.as_deref()) {
        let _ = app.emit("riven", &scan);
    }
    crate::overlay::show(app);
    if cached.is_some() {
        return;
    }
    let Some(slug) = card.weapon_slug.clone() else { return };
    let list = wfc_core::riven::riven_auctions(&st.http, &slug).await.unwrap_or_default();
    RIVEN_AUCTIONS
        .lock()
        .get_or_insert_with(Default::default)
        .insert(slug, (std::time::Instant::now(), list.clone()));
    if RIVEN_GEN.load(std::sync::atomic::Ordering::Relaxed) == gen {
        if let Ok(scan) = build_scan(app, card, Some(&list)) {
            let _ = app.emit("riven", &scan);
        }
    }
}

/// Read the riven card on screen, show it, then keep an eye on the screen:
/// a new roll updates the panel, no riven card any more closes it.
pub async fn riven_scan_now(app: &AppHandle) {
    use std::sync::atomic::Ordering;
    use tauri::Emitter;
    let gen = RIVEN_GEN.fetch_add(1, Ordering::Relaxed) + 1;
    let card = match read_card(app).await {
        Ok(Some(c)) => c,
        Ok(None) => {
            tracing::warn!("riven scan: no riven card found on screen");
            let _ = app.emit("overlay-error", "riven: no riven card on screen");
            return;
        }
        Err(e) => {
            tracing::warn!("riven scan: {e}");
            let _ = app.emit("overlay-error", format!("riven: {e}"));
            return;
        }
    };
    let mut last = card.clone();
    {
        let app = app.clone();
        tauri::async_runtime::spawn(async move { show_card(&app, gen, card).await });
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut misses = 0;
        for _ in 0..600 {
            tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
            if RIVEN_GEN.load(Ordering::Relaxed) != gen {
                return;
            }
            match read_card(&app).await {
                Ok(Some(c)) => {
                    misses = 0;
                    if !same_roll(&c, &last) {
                        last = c.clone();
                        show_card(&app, gen, c).await;
                    }
                }
                _ => {
                    misses += 1;
                    if misses >= 2 {
                        let _ = app.emit("riven-done", ());
                        return;
                    }
                }
            }
        }
    });
}

#[tauri::command]
pub async fn riven_scan(app: AppHandle) -> R<RivenScan> {
    let card = read_card(&app).await.map_err(err)?.ok_or("no riven card found on screen")?;
    let cached = card.weapon_slug.as_deref().and_then(cached_auctions);
    build_scan(&app, card, cached.as_deref()).map_err(err)
}

