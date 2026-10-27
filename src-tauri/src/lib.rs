mod commands;
mod overlay;

use parking_lot::RwLock;
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tokio::sync::broadcast;
use wfc_core::{db::Db, export::Export, log::GameEvent, market::Market};

pub struct AppState {
    pub http: reqwest::Client,
    pub db: Db,
    pub market: Market,
    pub export: Arc<RwLock<Option<Arc<Export>>>>,
    pub events: broadcast::Sender<GameEvent>,
    pub player: Arc<RwLock<Option<String>>>,
    pub current_relic: Arc<RwLock<Option<String>>>,
    pub ws: Arc<RwLock<Option<wfc_core::ws::Socket>>>,
    pub wfm_status: Arc<RwLock<Option<wfc_core::ws::RichStatus>>>,
    pub worldstate: Arc<RwLock<Option<serde_json::Value>>>,
    pub overlay_edit: Arc<RwLock<bool>>,
    pub hotkeys: Arc<RwLock<std::collections::HashMap<String, String>>>,
    pub hud_hidden: Arc<RwLock<bool>>,
}

/// (Re)connect the warframe.market socket with the current token and forward
/// its events to the UI. No-op without a token.
pub fn connect_ws(app: &tauri::AppHandle) {
    let st = app.state::<AppState>();
    let Some(token) = st.market.token() else { return };
    if let Some(old) = st.ws.write().take() {
        let h = old.clone();
        tauri::async_runtime::spawn(async move { h.close().await });
    }
    let (sock, driver) = wfc_core::ws::Socket::connect(token);
    let mut rx = sock.events.subscribe();
    *st.ws.write() = Some(sock);
    tauri::async_runtime::spawn(driver);
    let app2 = app.clone();
    tauri::async_runtime::spawn(async move {
        while let Ok(ev) = rx.recv().await {
            if let wfc_core::ws::WsEvent::Status(s) = &ev {
                *app2.state::<AppState>().wfm_status.write() = Some(s.clone());
            }
            let _ = app2.emit("wfm-ws", &ev);
        }
    });
}

pub fn show_main(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
    }
}

/// Tray icon: left click opens the window, the menu also quits. With
/// "launch with game" the app lives here between sessions.
fn tray(app: &tauri::App) -> tauri::Result<()> {
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
    let show = MenuItem::with_id(app, "show", "Show VoidTrace", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let mut b = TrayIconBuilder::new()
        .tooltip("VoidTrace")
        .menu(&Menu::with_items(app, &[&show, &quit])?)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, e| match e.id.as_ref() {
            "show" => show_main(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, e| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = e {
                show_main(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        b = b.icon(icon.clone());
    }
    b.build(app)?;
    Ok(())
}

pub fn run() {
    tracing_subscriber::fmt().with_env_filter("info,voidtrace=debug,wfc_core=debug").init();

    let http = reqwest::Client::builder()
        .user_agent(wfc_core::market::USER_AGENT)
        .build()
        .expect("http client");
    let db = Db::open().expect("db");
    let market = Market::new(http.clone());
    market.set_token(db.get("wfm_token"));
    let (events, _) = broadcast::channel::<GameEvent>(256);

    let state = AppState {
        http,
        db,
        market,
        export: Arc::new(RwLock::new(Export::load_cached().map(Arc::new))),
        events,
        player: Arc::new(RwLock::new(None)),
        current_relic: Arc::new(RwLock::new(None)),
        ws: Arc::new(RwLock::new(None)),
        wfm_status: Arc::new(RwLock::new(None)),
        worldstate: Arc::new(RwLock::new(None)),
        overlay_edit: Arc::new(RwLock::new(false)),
        hotkeys: Arc::new(RwLock::new(Default::default())),
        hud_hidden: Arc::new(RwLock::new(false)),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--background"]),
        ))
        .manage(state)
        .on_window_event(|w, e| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = e {
                if w.label() != "main" {
                    return;
                }
                // Launch-with-game keeps watching from the tray; otherwise the
                // hidden overlay window must not keep the process alive.
                if w.state::<AppState>().db.get("launch_with_game").as_deref() == Some("1") {
                    api.prevent_close();
                    let _ = w.hide();
                } else {
                    w.app_handle().exit(0);
                }
            }
        })
        .setup(|app| {
            let handle = app.handle().clone();
            let st = app.state::<AppState>();

            // Autostart passes --background: stay in the tray until the game starts.
            tray(app)?;
            if !std::env::args().any(|a| a == "--background") {
                show_main(&handle);
            }

            // EE.log tailer.
            let patterns = st
                .db
                .get("log_patterns")
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();
            let raw = st.db.get("log_raw").as_deref() == Some("1");
            wfc_core::log::spawn_tailer(wfc_core::ee_log_path(), patterns, st.events.clone(), raw)?;
            wfc_core::log::spawn_process_watcher(st.events.clone());

            // Forward events to the UI and drive the overlay.
            let mut rx = st.events.subscribe();
            let h2 = handle.clone();
            tauri::async_runtime::spawn(async move {
                while let Ok(ev) = rx.recv().await {
                    let _ = h2.emit("game-event", &ev);
                    overlay::on_event(&h2, ev).await;
                }
            });

            // Warm caches in the background.
            let h3 = handle.clone();
            tauri::async_runtime::spawn(async move {
                let st = h3.state::<AppState>();
                if let Err(e) = st.market.load_items().await {
                    tracing::warn!("market items: {e}");
                }
                match Export::fetch(&st.http).await {
                    Ok(ex) => *st.export.write() = Some(Arc::new(ex)),
                    Err(e) => tracing::warn!("export: {e}"),
                }
                let _ = h3.emit("ready", ());
            });

            // World state for the widgets, once a minute.
            let h4 = handle.clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    let st = h4.state::<AppState>();
                    match wfc_core::worldstate::fetch(&st.http).await {
                        Ok(v) => {
                            *st.worldstate.write() = Some(v.clone());
                            let _ = h4.emit("worldstate", &v);
                        }
                        Err(e) => tracing::warn!("worldstate: {e}"),
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                }
            });

            overlay::init(&handle)?;
            for (action, setting, default) in overlay::HOTKEYS {
                let key = st.db.get(setting).unwrap_or_else(|| default.to_string());
                if let Err(e) = overlay::register_hotkey(&handle, action, &key) {
                    tracing::warn!("{action} hotkey {key}: {e}");
                }
            }
            connect_ws(&handle);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::status,
            commands::refresh_data,
            commands::set_setting,
            commands::get_setting,
            commands::load_profile,
            commands::mastery,
            commands::relics_for,
            commands::relic_plan,
            commands::relic_list,
            commands::market_search,
            commands::market_items_by_id,
            commands::market_item,
            commands::set_launch_with_game,
            commands::game_running,
            commands::game_focused,
            commands::worldstate,
            commands::overlay_edit,
            commands::set_overlay_hotkey,
            commands::overlay_hotkeys,
            commands::hud_hidden,
            commands::set_hud_hidden,
            commands::market_price,
            commands::market_orders,
            commands::wfm_login,
            commands::wfm_logout,
            commands::wfm_my_orders,
            commands::wfm_closed_orders,
            commands::wfm_create_order,
            commands::wfm_update_order,
            commands::wfm_delete_order,
            commands::wfm_status,
            commands::wfm_set_status,
            commands::wfm_chats,
            commands::wfm_chat,
            commands::wfm_chat_send,
            commands::open_url,
            commands::inventory,
            commands::inventory_set,
            commands::inventory_add,
            commands::inventory_sync,
            commands::grade_riven,
            commands::trades,
            commands::add_trade,
            commands::record_trade,
            commands::delete_trade,
            commands::datapoints,
            commands::add_datapoint,
            commands::relic_runs,
            commands::ocr_now,
            commands::overlay_toggle,
            commands::overlay_preview,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
