//! Phone / tablet dashboard: a small read-only web page served on the local
//! network while it is switched on. The link (and its QR code) carries a
//! random token that changes on every start; without it nothing answers.
//! Changes are pushed to the page as server-sent events.

use crate::AppState;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::LazyLock;
use tauri::{AppHandle, Listener, Manager};
use tokio::sync::{broadcast, oneshot};

const PAGE: &str = include_str!("dashboard.html");
/// App events the page cares about; forwarded as they happen.
const EVENTS: &[&str] = &["worldstate", "rewards", "rewards-done", "notice", "game-event", "inventory-synced"];

struct Running {
    url: String,
    qr: String,
    stop: oneshot::Sender<()>,
    listeners: Vec<tauri::EventId>,
}

static RUNNING: LazyLock<Mutex<Option<Running>>> = LazyLock::new(Default::default);
static REWARDS: LazyLock<Mutex<Value>> = LazyLock::new(|| Mutex::new(Value::Null));

#[derive(Serialize, Clone, Default)]
pub struct DashInfo {
    pub running: bool,
    pub url: Option<String>,
    /// QR code of the URL as an SVG document.
    pub qr: Option<String>,
}

#[derive(Clone)]
struct Ctx {
    app: AppHandle,
    token: String,
    tx: broadcast::Sender<String>,
}

#[derive(Deserialize)]
struct Auth {
    t: Option<String>,
}

impl Ctx {
    fn ok(&self, a: &Auth) -> bool {
        a.t.as_deref() == Some(self.token.as_str())
    }
}

/// The address other devices reach this PC on. Connecting a UDP socket sends
/// nothing; it only asks the OS which interface it would use.
fn lan_ip() -> Option<std::net::IpAddr> {
    let s = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    s.connect("192.0.2.1:9").ok()?;
    let ip = s.local_addr().ok()?.ip();
    (!ip.is_loopback() && !ip.is_unspecified()).then_some(ip)
}

pub fn info() -> DashInfo {
    match RUNNING.lock().as_ref() {
        Some(r) => DashInfo { running: true, url: Some(r.url.clone()), qr: Some(r.qr.clone()) },
        None => DashInfo::default(),
    }
}

pub async fn start(app: &AppHandle, port: u16) -> anyhow::Result<DashInfo> {
    stop(app);
    let ip = lan_ip().ok_or_else(|| anyhow::anyhow!("no local network found"))?;
    let token = uuid::Uuid::new_v4().simple().to_string();
    let listener = tokio::net::TcpListener::bind((ip, port)).await?;
    let addr = listener.local_addr()?;
    let url = format!("http://{addr}/?t={token}");
    let qr = qrcode::QrCode::new(url.as_bytes())?
        .render::<qrcode::render::svg::Color>()
        .min_dimensions(240, 240)
        .dark_color(qrcode::render::svg::Color("#000000"))
        .light_color(qrcode::render::svg::Color("#ffffff"))
        .build();

    let (tx, _) = broadcast::channel::<String>(64);
    let mut listeners = vec![];
    for ev in EVENTS {
        let tx = tx.clone();
        let name = ev.to_string();
        listeners.push(app.listen_any(*ev, move |e| {
            let payload: Value = serde_json::from_str(e.payload()).unwrap_or(Value::Null);
            match name.as_str() {
                "rewards" => *REWARDS.lock() = payload.clone(),
                "rewards-done" => *REWARDS.lock() = Value::Null,
                _ => {}
            }
            let _ = tx.send(json!({ "kind": name, "payload": payload }).to_string());
        }));
    }

    let ctx = Ctx { app: app.clone(), token, tx };
    let router = Router::new()
        .route("/", get(page))
        .route("/api/state", get(state))
        .route("/api/events", get(events))
        .with_state(ctx);
    let (stop_tx, stop_rx) = oneshot::channel::<()>();
    tauri::async_runtime::spawn(async move {
        let serve = axum::serve(listener, router).with_graceful_shutdown(async {
            let _ = stop_rx.await;
        });
        if let Err(e) = serve.await {
            tracing::warn!("dashboard: {e}");
        }
    });
    tracing::info!("phone dashboard on http://{addr}/");
    *RUNNING.lock() = Some(Running { url, qr, stop: stop_tx, listeners });
    Ok(info())
}

pub fn stop(app: &AppHandle) {
    if let Some(r) = RUNNING.lock().take() {
        let _ = r.stop.send(());
        for id in r.listeners {
            app.unlisten(id);
        }
        tracing::info!("phone dashboard stopped");
    }
}

async fn page(State(c): State<Ctx>, Query(a): Query<Auth>) -> Response {
    if !c.ok(&a) {
        return (StatusCode::FORBIDDEN, "Open the link from VoidTrace (Settings > Phone dashboard).").into_response();
    }
    Html(PAGE).into_response()
}

async fn state(State(c): State<Ctx>, Query(a): Query<Auth>) -> Response {
    if !c.ok(&a) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let st = c.app.state::<AppState>();
    let now = chrono::Local::now();
    let day0 = now.date_naive().and_hms_opt(0, 0, 0).and_then(|d| d.and_local_timezone(chrono::Local).single()).map(|d| d.timestamp()).unwrap_or(0);
    let trades = st.db.trades().unwrap_or_default();
    let today: Vec<_> = trades.iter().filter(|t| t.ts >= day0).collect();
    let sold: i64 = today.iter().filter(|t| t.direction == "sell").map(|t| t.platinum as i64).sum();
    let bought: i64 = today.iter().filter(|t| t.direction == "buy").map(|t| t.platinum as i64).sum();
    let points = st.db.datapoints(1).unwrap_or_default();
    let body = json!({
        "world": st.worldstate.read().clone(),
        "rewards": REWARDS.lock().clone(),
        "trading": { "sold": sold, "bought": bought, "count": today.len(), "last": trades.iter().take(6).collect::<Vec<_>>() },
        "listings": crate::watch::last(),
        "inventory": points.first(),
        "notices": crate::notify::recent().into_iter().take(6).collect::<Vec<_>>(),
        "player": st.player.read().clone().or_else(|| st.db.get("player_name")),
        "game": wfc_core::memory::find_pid().is_some(),
    });
    Json(body).into_response()
}

async fn events(State(c): State<Ctx>, Query(a): Query<Auth>) -> Response {
    if !c.ok(&a) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let rx = c.tx.subscribe();
    let stream = futures_util::stream::unfold(rx, |mut rx| async move {
        loop {
            match rx.recv().await {
                Ok(msg) => return Some((Ok::<Event, std::convert::Infallible>(Event::default().data(msg)), rx)),
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(_) => return None,
            }
        }
    });
    Sse::new(stream).keep_alive(KeepAlive::default()).into_response()
}