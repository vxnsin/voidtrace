//! Overlay window: transparent, always on top, click-through. Shown on the
//! relic reward screen with OCR results, hidden after the pick.
//!
//! Warframe must run in Borderless Fullscreen (exclusive fullscreen hides every
//! overlay that is not injected, Overwolf included).

use crate::AppState;
use tauri::{AppHandle, Emitter, Manager};
use wfc_core::log::GameEvent;
use wfc_core::ocr::Crop;

pub fn init(app: &AppHandle) -> tauri::Result<()> {
    if let Some(w) = app.get_webview_window("overlay") {
        w.set_ignore_cursor_events(true)?;
    }
    Ok(())
}

pub fn show(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("overlay") {
        let _ = w.show();
        let _ = w.set_always_on_top(true);
    }
}

pub fn hide(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("overlay") {
        let _ = w.hide();
    }
}

pub async fn run_ocr(app: &AppHandle) {
    let st = app.state::<AppState>();
    let crop: Crop = st
        .db
        .get("ocr_crop")
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    // "Got rewards" fires once the cards are built; a short delay covers the fade-in.
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    match wfc_core::ocr::read_rewards(&st.market, crop).await {
        Ok(hits) => {
            let names: Vec<String> = hits.iter().map(|h| h.raw.clone()).collect();
            let relic = st.current_relic.read().clone().unwrap_or_default();
            let _ = st.db.add_relic_run(&relic, &names, None);
            let _ = app.emit("rewards", &hits);
            show(app);
        }
        Err(e) => {
            tracing::warn!("ocr: {e}");
            let _ = app.emit("overlay-error", e.to_string());
        }
    }
}

/// Memory scan -> ledger + datapoint. Opt-in via the `memory_sync` setting.
pub async fn sync_inventory(app: &AppHandle) -> anyhow::Result<crate::commands::SyncResult> {
    let st = app.state::<AppState>();
    let t = std::time::Instant::now();
    let v = tokio::task::spawn_blocking(wfc_core::memory::read_inventory_json).await??;
    let ex = st.export.read().clone().ok_or_else(|| anyhow::anyhow!("export not loaded yet"))?;
    let snap = wfc_core::inventory::parse(&ex, &v);
    let mut rows = Vec::new();
    for it in &snap.items {
        if let Some(mi) = st.market.find_item_exact(&it.name) {
            if mi.tradable {
                rows.push((mi.slug, mi.name, it.count.min(i32::MAX as i64) as i32));
            }
        }
    }
    st.db.inventory_replace(&rows)?;
    st.db.add_datapoint(&wfc_core::db::Datapoint {
        ts: chrono::Utc::now().timestamp(),
        platinum: snap.platinum as i32,
        ducats: snap.ducats as i32,
        credits: snap.credits,
    })?;
    let _ = st.db.set("inventory_synced_at", &chrono::Utc::now().timestamp().to_string());
    let res = crate::commands::SyncResult {
        platinum: snap.platinum,
        credits: snap.credits,
        ducats: snap.ducats,
        items_total: snap.items.len(),
        items_tradeable: rows.len(),
        seconds: t.elapsed().as_secs_f32(),
    };
    let _ = app.emit("inventory-synced", &res);
    Ok(res)
}

/// The game parses the inventory document right after receiving it and frees
/// the buffer, which then gets overwritten over the next minutes. So: scan as
/// soon as a loading screen is over and keep retrying for a while.
fn auto_sync(app: &AppHandle, initial_delay_secs: u64) {
    let st = app.state::<AppState>();
    if st.db.get("memory_sync").as_deref() != Some("1") {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(initial_delay_secs)).await;
        let mut last_err = String::new();
        for attempt in 0..20 {
            match sync_inventory(&app).await {
                Ok(r) => {
                    tracing::info!("inventory synced on attempt {attempt}: {} items", r.items_total);
                    return;
                }
                Err(e) => {
                    last_err = e.to_string();
                    if last_err.contains("not running") || last_err.contains("cannot open") {
                        break;
                    }
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        }
        tracing::warn!("inventory sync gave up: {last_err}");
        let _ = app.emit("inventory-sync-error", last_err);
    });
}

/// Mirror the game state to the warframe.market status, if enabled.
fn auto_activity(app: &AppHandle, kind: &str, details: Option<String>) {
    let st = app.state::<AppState>();
    if st.db.get("wfm_auto_activity").as_deref() == Some("0") {
        return;
    }
    let Some(sock) = st.ws.read().clone() else { return };
    let kind = kind.to_string();
    tauri::async_runtime::spawn(async move {
        let act = wfc_core::ws::Activity { kind, details, started_at: chrono::Utc::now().timestamp().into() };
        if let Err(e) = sock.set_status("ingame", None, Some(act)).await {
            tracing::warn!("wfm status: {e}");
        }
    });
}

pub async fn on_event(app: &AppHandle, ev: GameEvent) {
    let st = app.state::<AppState>();
    match &ev {
        GameEvent::LoggedIn { .. } => auto_activity(app, "IN_ORBITER", None),
        GameEvent::MissionStart { name } => auto_activity(app, "ON_MISSION", Some(name.clone())),
        GameEvent::MissionEnd => auto_activity(app, "IN_ORBITER", None),
        _ => {}
    }
    match ev {
        GameEvent::LoggedIn { name, id } => {
            *st.player.write() = Some(name.clone());
            if st.db.get("player_name").is_none() {
                let _ = st.db.set("player_name", &name);
            }
            if let Some(id) = id {
                if st.db.get("player_id").is_none() {
                    let _ = st.db.set("player_id", &id);
                }
            }
            // The inventory arrives right after login.
            auto_sync(app, 4);
        }
        GameEvent::MissionStart { .. } => auto_sync(app, 6),
        GameEvent::MissionEnd => {
            auto_sync(app, 4);
            hide(app);
        }
        GameEvent::RelicOpened { relic } => {
            let strip = |u: &str| {
                u.trim_end_matches("Bronze")
                    .trim_end_matches("Silver")
                    .trim_end_matches("Gold")
                    .trim_end_matches("Platinum")
                    .to_string()
            };
            let name = st.export.read().as_ref().and_then(|ex| {
                ex.relics
                    .iter()
                    .find(|r| strip(&r.unique_name).ends_with(&relic))
                    .map(|r| r.name.clone())
            });
            *st.current_relic.write() = Some(name.unwrap_or(relic));
        }
        GameEvent::RewardScreen => {
            if st.db.get("overlay_enabled").as_deref() != Some("0") {
                run_ocr(app).await;
            }
        }
        GameEvent::RewardDone => hide(app),
        _ => {}
    }
}
