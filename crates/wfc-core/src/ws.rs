//! warframe.market realtime socket (wss://ws.warframe.market/socket, subprotocol
//! `wfm`): sign in with the v1 JWT, set status/activity, receive events.
//! Unknown routes are forwarded raw so new server events can be discovered.

use anyhow::{bail, Context, Result};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use parking_lot::Mutex;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;

const URL: &str = "wss://ws.warframe.market/socket";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Activity {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    #[serde(rename = "startedAt")]
    pub started_at: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RichStatus {
    pub status: String,
    #[serde(default, rename = "statusSetAt")]
    pub status_set_at: Option<String>,
    #[serde(default, rename = "statusUntil")]
    pub status_until: Option<String>,
    #[serde(default)]
    pub activity: Option<Activity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WsEvent {
    Connected,
    Disconnected { reason: String },
    SignedIn,
    Status(RichStatus),
    /// A new order anywhere on the market (after `subscribe`); the payload is
    /// the order with its `user`, shaped like `market::Order`.
    NewOrder(Value),
    Raw { route: String, payload: Value },
}

pub enum Command {
    SetStatus { status: String, duration: Option<u32>, activity: Option<Activity> },
    Send(Value),
    Close,
}

#[derive(Clone)]
pub struct Socket {
    tx: mpsc::Sender<Command>,
    pub events: broadcast::Sender<WsEvent>,
    /// Subscribe messages, sent again after every reconnect.
    subs: Arc<Mutex<Vec<Value>>>,
}

/// Every new order on the market, for price alerts (no item filter on the
/// server side; we filter).
pub fn new_orders_sub() -> Value {
    json!({ "route": "@wfm|cmd/subscribe/newOrders", "payload": { "platform": "pc", "crossplay": true } })
}

impl Socket {
    /// Builds the socket handle and the driver future, which connects, signs
    /// in (when there is a token) and keeps reconnecting until `Close`. The
    /// caller spawns the future on its runtime (this crate does not assume one
    /// is current).
    pub fn connect(token: Option<String>) -> (Socket, impl std::future::Future<Output = ()> + Send) {
        let (tx, mut rx) = mpsc::channel::<Command>(32);
        let (events, _) = broadcast::channel::<WsEvent>(256);
        let ev = events.clone();
        let subs: Arc<Mutex<Vec<Value>>> = Arc::default();
        let subs2 = subs.clone();
        let driver = async move {
            let mut backoff = 2u64;
            loop {
                let resend = subs2.lock().clone();
                match run(token.as_deref(), &resend, &mut rx, &ev).await {
                    Ok(()) => break, // closed on purpose
                    Err(e) => {
                        let _ = ev.send(WsEvent::Disconnected { reason: e.to_string() });
                        tracing::warn!("wfm ws: {e}; reconnecting in {backoff}s");
                    }
                }
                tokio::time::sleep(std::time::Duration::from_secs(backoff)).await;
                backoff = (backoff * 2).min(60);
            }
        };
        (Socket { tx, events, subs }, driver)
    }

    /// Send a subscribe message now and after every reconnect (once per route).
    pub async fn subscribe(&self, msg: Value) -> Result<()> {
        {
            let mut s = self.subs.lock();
            if s.iter().any(|m| m["route"] == msg["route"]) {
                return Ok(());
            }
            s.push(msg.clone());
        }
        self.send_raw(msg).await
    }

    pub async fn set_status(&self, status: &str, duration: Option<u32>, activity: Option<Activity>) -> Result<()> {
        self.tx
            .send(Command::SetStatus { status: status.to_string(), duration, activity })
            .await
            .map_err(|_| anyhow::anyhow!("socket closed"))
    }

    pub async fn send_raw(&self, msg: Value) -> Result<()> {
        self.tx.send(Command::Send(msg)).await.map_err(|_| anyhow::anyhow!("socket closed"))
    }

    pub async fn close(&self) {
        let _ = self.tx.send(Command::Close).await;
    }
}

async fn run(token: Option<&str>, subs: &[Value], rx: &mut mpsc::Receiver<Command>, ev: &broadcast::Sender<WsEvent>) -> Result<()> {
    let mut req = URL.into_client_request()?;
    req.headers_mut().insert("Sec-WebSocket-Protocol", "wfm".parse()?);
    req.headers_mut().insert("User-Agent", crate::market::USER_AGENT.parse()?);
    let (ws, _) = tokio_tungstenite::connect_async(req).await.context("connect")?;
    let (mut sink, mut stream) = ws.split();
    let _ = ev.send(WsEvent::Connected);

    if let Some(token) = token {
        sink.send(Message::Text(
            json!({ "route": "@wfm|cmd/auth/signIn", "id": "signin", "payload": { "token": token } }).to_string(),
        ))
        .await?;
    }
    for s in subs {
        sink.send(Message::Text(s.to_string())).await?;
    }

    let mut seq = 0u64;
    loop {
        tokio::select! {
            cmd = rx.recv() => {
                let Some(cmd) = cmd else { return Ok(()) };
                match cmd {
                    Command::Close => { let _ = sink.send(Message::Close(None)).await; return Ok(()); }
                    Command::Send(v) => { sink.send(Message::Text(v.to_string())).await?; }
                    Command::SetStatus { status, duration, activity } => {
                        seq += 1;
                        let mut payload = json!({ "status": status });
                        if let Some(d) = duration { payload["duration"] = d.into(); } else { payload["duration"] = Value::Null; }
                        payload["activity"] = activity.map(|a| serde_json::to_value(a).unwrap()).unwrap_or(Value::Null);
                        sink.send(Message::Text(json!({
                            "route": "@wfm|cmd/status/set", "id": format!("status-{seq}"), "payload": payload
                        }).to_string())).await?;
                    }
                }
            }
            msg = stream.next() => {
                let Some(msg) = msg else { bail!("server closed the connection") };
                match msg? {
                    Message::Text(t) => handle(&t, ev),
                    Message::Ping(p) => { sink.send(Message::Pong(p)).await?; }
                    Message::Close(f) => bail!("closed: {f:?}"),
                    _ => {}
                }
            }
        }
    }
}

fn handle(text: &str, ev: &broadcast::Sender<WsEvent>) {
    let Ok(v) = serde_json::from_str::<Value>(text) else { return };
    let route = v["route"].as_str().unwrap_or("").to_string();
    let payload = v.get("payload").cloned().unwrap_or(Value::Null);
    let out = match route.as_str() {
        "@wfm|cmd/auth/signIn:ok" => WsEvent::SignedIn,
        "@wfm|cmd/auth/signIn:error" => WsEvent::Disconnected { reason: format!("sign-in rejected: {payload}") },
        "@wfm|event/status/set" | "@wfm|cmd/status/set:ok" => match serde_json::from_value::<RichStatus>(payload.clone()) {
            Ok(s) => WsEvent::Status(s),
            Err(_) => WsEvent::Raw { route, payload },
        },
        "@wfm|event/subscriptions/newOrder" => WsEvent::NewOrder(payload),
        _ => WsEvent::Raw { route, payload },
    };
    let _ = ev.send(out);
}
