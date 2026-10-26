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

pub fn run() {
    tracing_subscriber::fmt().with_env_filter("info,voidtrace=debug,wfc_core=debug").init();

    let http = reqwest::Client::builder()
        .user_agent("VoidTrace/0.1 (+https://github.com/vxnsin)")
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
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(state)
        .setup(|app| {
            let handle = app.handle().clone();
            let st = app.state::<AppState>();

            // EE.log tailer.
            let patterns = st
                .db
                .get("log_patterns")
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();
            let raw = st.db.get("log_raw").as_deref() == Some("1");
            wfc_core::log::spawn_tailer(wfc_core::ee_log_path(), patterns, st.events.clone(), raw)?;

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

            overlay::init(&handle)?;
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
            commands::market_price,
            commands::market_orders,
            commands::wfm_login,
            commands::wfm_logout,
            commands::wfm_my_orders,
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
