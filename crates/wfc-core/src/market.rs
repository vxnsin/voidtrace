//! warframe.market API v2 client.

use anyhow::{bail, Context, Result};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

const BASE: &str = "https://api.warframe.market/v2";
// v2 /auth/signin is first-party only (Firebase App Check); third-party
// clients keep using the v1 login and send its JWT to v2 endpoints.
const V1_SIGNIN: &str = "https://api.warframe.market/v1/auth/signin";
/// warframe.market asks every client for a descriptive User-Agent with a way to
/// reach the author (docs.warframe.market, Rules).
pub const USER_AGENT: &str = concat!("VoidTrace/", env!("CARGO_PKG_VERSION"), " (+https://github.com/vxnsin/voidtrace)");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketItem {
    pub id: String,
    pub slug: String,
    pub name: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub ducats: u32,
    #[serde(default)]
    pub tradable: bool,
    #[serde(default)]
    pub tags: Vec<String>,
    /// Prime parts and relics out of the drop tables (warframe.market flag).
    #[serde(default)]
    pub vaulted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub platinum: u32,
    pub quantity: u32,
    #[serde(default)]
    pub visible: bool,
    #[serde(rename = "itemId")]
    pub item_id: String,
    #[serde(default)]
    pub rank: Option<u8>,
    #[serde(default)]
    pub user: Option<OrderUser>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderUser {
    #[serde(rename = "ingameName")]
    pub ingame_name: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub reputation: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PriceSummary {
    pub slug: String,
    pub name: String,
    pub ducats: u32,
    /// Lowest visible sell order from an online/ingame user.
    pub sell: Option<u32>,
    /// Highest visible buy order.
    pub buy: Option<u32>,
    pub sell_count: usize,
    #[serde(default)]
    pub vaulted: bool,
}

/// A finished order from a user's public warframe.market statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClosedOrder {
    pub id: String,
    /// "sell" or "buy"
    pub kind: String,
    pub quantity: u32,
    /// Per unit.
    pub platinum: f64,
    pub closed: String,
    pub slug: String,
    pub name: String,
}

/// What v2 returns when an order is closed: the sale/purchase it recorded
/// (docs.warframe.market, data model "Transaction"; no itemId, only the order).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: String,
    #[serde(default, rename = "type")]
    pub kind: String,
    #[serde(default, rename = "originId")]
    pub origin_id: String,
    #[serde(default)]
    pub platinum: f64,
    #[serde(default)]
    pub quantity: u32,
    #[serde(default, rename = "createdAt")]
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatUser {
    pub id: String,
    #[serde(default)]
    pub ingame_name: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub avatar: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chat {
    pub id: String,
    #[serde(default)]
    pub chat_with: Vec<ChatUser>,
    #[serde(default)]
    pub unread_count: u32,
    #[serde(default)]
    pub last_update: String,
    #[serde(default)]
    pub messages: Vec<ChatMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub message_from: String,
    #[serde(default)]
    pub send_date: String,
    #[serde(default)]
    pub raw_message: Option<String>,
}

#[derive(Deserialize)]
struct Envelope<T> {
    data: Option<T>,
    error: Option<serde_json::Value>,
}

#[derive(Clone)]
pub struct Market {
    http: reqwest::Client,
    token: Arc<RwLock<Option<String>>>,
    scheme: Arc<RwLock<&'static str>>,
    pub user_id: Arc<RwLock<Option<String>>>,
    items: Arc<RwLock<Vec<MarketItem>>>,
    price_cache: Arc<RwLock<HashMap<String, (std::time::Instant, PriceSummary)>>>,
}

impl Market {
    pub fn new(http: reqwest::Client) -> Self {
        Self {
            http,
            token: Arc::new(RwLock::new(None)),
            scheme: Arc::new(RwLock::new("Bearer")),
            user_id: Arc::new(RwLock::new(None)),
            items: Arc::new(RwLock::new(Vec::new())),
            price_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn set_token(&self, t: Option<String>) {
        *self.token.write() = t;
    }
    pub fn token(&self) -> Option<String> {
        self.token.read().clone()
    }
    pub fn items(&self) -> Vec<MarketItem> {
        self.items.read().clone()
    }

    fn req(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        let mut r = self
            .http
            .request(method, format!("{BASE}{path}"))
            .header("Platform", "pc")
            .header("Language", "en")
            .header("Crossplay", "true")
            .header("User-Agent", USER_AGENT);
        if let Some(t) = self.token.read().as_ref() {
            r = r.header("Authorization", format!("{} {t}", self.scheme.read()));
        }
        r
    }

    /// Authenticated v2 call. v1 tokens are documented to work with the
    /// `Bearer` scheme; if the server rejects that, retry once with `JWT`
    /// (the scheme v1 itself uses) and remember the working one.
    async fn authed<T: for<'a> Deserialize<'a>>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> Result<T> {
        for attempt in 0..2 {
            let mut r = self.req(method.clone(), path);
            if let Some(b) = &body {
                r = r.json(b);
            }
            let res = r.send().await?;
            let status = res.status();
            let text = res.text().await?;
            let unauthorized = status.as_u16() == 401 || text.contains("app.errors.unauthorized");
            if unauthorized && attempt == 0 && *self.scheme.read() == "Bearer" {
                *self.scheme.write() = "JWT";
                continue;
            }
            let env: Envelope<T> = serde_json::from_str(&text)
                .with_context(|| format!("market {status}: {}", &text[..text.len().min(300)]))?;
            if let Some(e) = env.error {
                bail!("warframe.market: {e}");
            }
            return env.data.context("empty data");
        }
        unreachable!()
    }

    async fn unwrap<T: for<'a> Deserialize<'a>>(res: reqwest::Response) -> Result<T> {
        let status = res.status();
        let text = res.text().await?;
        let env: Envelope<T> = serde_json::from_str(&text)
            .with_context(|| format!("market {status}: {}", &text[..text.len().min(300)]))?;
        if let Some(e) = env.error {
            bail!("warframe.market: {e}");
        }
        env.data.context("empty data")
    }

    pub async fn load_items(&self) -> Result<usize> {
        #[derive(Deserialize)]
        struct Raw {
            id: String,
            slug: String,
            #[serde(default)]
            ducats: Option<u32>,
            #[serde(default)]
            tradable: Option<bool>,
            #[serde(default)]
            tags: Vec<String>,
            #[serde(default)]
            vaulted: Option<bool>,
            i18n: HashMap<String, I18n>,
        }
        #[derive(Deserialize)]
        struct I18n {
            name: String,
            #[serde(default)]
            icon: String,
        }
        let raw: Vec<Raw> = Self::unwrap(self.req(reqwest::Method::GET, "/items").send().await?).await?;
        let items: Vec<MarketItem> = raw
            .into_iter()
            .filter_map(|r| {
                let en = r.i18n.get("en")?;
                Some(MarketItem {
                    id: r.id,
                    slug: r.slug,
                    name: en.name.clone(),
                    icon: en.icon.clone(),
                    ducats: r.ducats.unwrap_or(0),
                    tradable: r.tradable.unwrap_or(true),
                    tags: r.tags,
                    vaulted: r.vaulted.unwrap_or(false),
                })
            })
            .collect();
        let n = items.len();
        *self.items.write() = items;
        Ok(n)
    }

    /// Exact (or word-order-insensitive) match only; no fuzzy fallback.
    pub fn find_item_exact(&self, name: &str) -> Option<MarketItem> {
        let n = crate::norm(name);
        let items = self.items.read();
        if let Some(i) = items.iter().find(|i| crate::norm(&i.name) == n) {
            return Some(i.clone());
        }
        let sorted = |s: &str| {
            let mut w: Vec<&str> = s.split(' ').collect();
            w.sort_unstable();
            w.join(" ")
        };
        let ns = sorted(&n);
        items.iter().find(|i| sorted(&crate::norm(&i.name)) == ns).cloned()
    }

    pub fn find_item(&self, name: &str) -> Option<MarketItem> {
        let n = crate::norm(name);
        let items = self.items.read();
        if let Some(i) = items.iter().find(|i| crate::norm(&i.name) == n) {
            return Some(i.clone());
        }
        // Same words, different order ("Prime Helios Cerebrum" vs "Helios Prime Cerebrum").
        let sorted = |s: &str| {
            let mut w: Vec<&str> = s.split(' ').collect();
            w.sort_unstable();
            w.join(" ")
        };
        let ns = sorted(&n);
        if let Some(i) = items.iter().find(|i| sorted(&crate::norm(&i.name)) == ns) {
            return Some(i.clone());
        }
        // Fuzzy (OCR typos): best Jaro-Winkler over the catalogue. Lengths must
        // be close too, or "forma blueprint" lands on "form up".
        items
            .iter()
            .map(|i| (crate::norm(&i.name), i))
            .filter(|(c, _)| {
                let (a, b) = (c.len() as f32, n.len() as f32);
                a.min(b) / a.max(b) > 0.75
            })
            .map(|(c, i)| (strsim::jaro_winkler(&n, &c), i))
            .filter(|(s, _)| *s > 0.9)
            .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap())
            .map(|(_, i)| i.clone())
    }

    pub async fn orders(&self, slug: &str) -> Result<Vec<Order>> {
        Self::unwrap(
            self.req(reqwest::Method::GET, &format!("/orders/item/{slug}"))
                .send()
                .await?,
        )
        .await
    }

    pub async fn price(&self, slug: &str) -> Result<PriceSummary> {
        if let Some((t, p)) = self.price_cache.read().get(slug) {
            if t.elapsed().as_secs() < 300 {
                return Ok(p.clone());
            }
        }
        let item = self
            .items
            .read()
            .iter()
            .find(|i| i.slug == slug)
            .cloned()
            .context("unknown slug")?;
        let orders = self.orders(slug).await?;
        let online = |o: &Order| {
            o.visible
                && o.user
                    .as_ref()
                    .map(|u| u.status == "ingame" || u.status == "online")
                    .unwrap_or(false)
        };
        let mut sells: Vec<u32> = orders
            .iter()
            .filter(|o| o.kind == "sell" && online(o))
            .map(|o| o.platinum)
            .collect();
        sells.sort_unstable();
        let buy = orders
            .iter()
            .filter(|o| o.kind == "buy" && online(o))
            .map(|o| o.platinum)
            .max();
        let p = PriceSummary {
            slug: slug.to_string(),
            name: item.name,
            ducats: item.ducats,
            sell: sells.first().copied(),
            buy,
            sell_count: sells.len(),
            vaulted: item.vaulted,
        };
        self.price_cache
            .write()
            .insert(slug.to_string(), (std::time::Instant::now(), p.clone()));
        Ok(p)
    }

    pub async fn login(&self, email: &str, password: &str) -> Result<String> {
        // v1 wants an (empty) JWT header present and auth_type=header to
        // return the token in the response header instead of a cookie.
        let res = self
            .http
            .post(V1_SIGNIN)
            .header("Authorization", "JWT ")
            .header("Platform", "pc")
            .header("Language", "en")
            .header("User-Agent", USER_AGENT)
            .json(&serde_json::json!({
                "email": email, "password": password, "auth_type": "header"
            }))
            .send()
            .await?;
        let auth = res
            .headers()
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);
        let status = res.status();
        let body: serde_json::Value = res.json().await.unwrap_or_default();
        if let Some(e) = body.get("error") {
            let msg = match e.pointer("/email/0").or(e.pointer("/password/0")).and_then(|x| x.as_str()) {
                Some("app.account.email_not_exist") => "no warframe.market account with this email".to_string(),
                Some("app.account.password_invalid") => "wrong password".to_string(),
                _ => e.to_string(),
            };
            bail!("warframe.market: {msg}");
        }
        let auth = auth.with_context(|| format!("login {status}: no authorization header"))?;
        let token = auth.trim_start_matches("JWT ").trim().to_string();
        let name = body
            .pointer("/payload/user/ingame_name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if let Some(id) = body.pointer("/payload/user/id").and_then(|v| v.as_str()) {
            *self.user_id.write() = Some(id.to_string());
        }
        *self.scheme.write() = "Bearer";
        self.set_token(Some(token));
        Ok(name)
    }

    /// v1 chat API (im = instant messaging). Not part of the public v2 docs;
    /// the endpoints still answer, so this may break without notice.
    async fn v1<T: for<'a> Deserialize<'a>>(&self, method: reqwest::Method, path: &str, body: Option<Value>) -> Result<T> {
        let token = self.token.read().clone().context("not logged in")?;
        let mut r = self
            .http
            .request(method, format!("https://api.warframe.market/v1{path}"))
            .header("Authorization", format!("JWT {token}"))
            .header("Platform", "pc")
            .header("Language", "en")
            .header("User-Agent", USER_AGENT);
        if let Some(b) = body {
            r = r.json(&b);
        }
        let res = r.send().await?;
        let status = res.status();
        let text = res.text().await?;
        let v: Value = serde_json::from_str(&text).with_context(|| format!("v1 {status}: {}", &text[..text.len().min(200)]))?;
        if let Some(e) = v.get("error") {
            bail!("warframe.market v1 ({status}): {e}");
        }
        serde_json::from_value(v["payload"].clone()).context("unexpected v1 payload")
    }

    pub async fn chats(&self) -> Result<Vec<Chat>> {
        #[derive(Deserialize)]
        struct P {
            chats: Vec<Chat>,
        }
        let p: P = self.v1(reqwest::Method::GET, "/im/chats", None).await?;
        Ok(p.chats)
    }

    pub async fn chat_messages(&self, chat_id: &str) -> Result<Vec<ChatMessage>> {
        #[derive(Deserialize)]
        struct P {
            messages: Vec<ChatMessage>,
        }
        let p: P = self.v1(reqwest::Method::GET, &format!("/im/chats/{chat_id}"), None).await?;
        Ok(p.messages)
    }

    pub async fn chat_send(&self, chat_id: &str, message: &str) -> Result<()> {
        let _: Value = self
            .v1(reqwest::Method::POST, &format!("/im/chats/{chat_id}"), Some(json!({ "message": message })))
            .await?;
        Ok(())
    }

    /// Every closed order of a user, from the public profile statistics (v1, not in
    /// the v2 docs; the website's statistics page uses it). No login needed.
    pub async fn closed_orders(&self, user: &str) -> Result<Vec<ClosedOrder>> {
        let res = self
            .http
            .get(format!("https://api.warframe.market/v1/profile/{}/statistics", user.to_lowercase()))
            .header("Platform", "pc")
            .header("Language", "en")
            .header("User-Agent", USER_AGENT)
            .send()
            .await?;
        let status = res.status();
        let v: Value = res.json().await.with_context(|| format!("statistics {status}"))?;
        let list = v.pointer("/payload/closed_orders").and_then(Value::as_array).context("no closed_orders in statistics")?;
        Ok(list
            .iter()
            .filter_map(|o| {
                Some(ClosedOrder {
                    id: o["id"].as_str()?.to_string(),
                    kind: o["order_type"].as_str()?.to_string(),
                    quantity: o["quantity"].as_u64().unwrap_or(1) as u32,
                    platinum: o["platinum"].as_f64().unwrap_or(0.0),
                    closed: o["closed_date"].as_str().unwrap_or("").to_string(),
                    slug: o.pointer("/item/url_name")?.as_str()?.to_string(),
                    name: o.pointer("/item/en/item_name").and_then(Value::as_str).unwrap_or("").to_string(),
                })
            })
            .collect())
    }

    pub async fn my_orders(&self) -> Result<Vec<Order>> {
        self.authed(reqwest::Method::GET, "/orders/my", None).await
    }

    pub async fn create_order(
        &self,
        item_id: &str,
        sell: bool,
        platinum: u32,
        quantity: u32,
        rank: Option<u8>,
    ) -> Result<Order> {
        let mut body = serde_json::json!({
            "itemId": item_id,
            "type": if sell { "sell" } else { "buy" },
            "platinum": platinum,
            "quantity": quantity,
            "visible": true,
        });
        if let Some(r) = rank {
            body["rank"] = r.into();
        }
        self.authed(reqwest::Method::POST, "/order", Some(body)).await
    }

    pub async fn update_order(&self, id: &str, platinum: u32, quantity: u32, visible: bool) -> Result<Order> {
        self.authed(
            reqwest::Method::PATCH,
            &format!("/order/{id}"),
            Some(serde_json::json!({ "platinum": platinum, "quantity": quantity, "visible": visible })),
        )
        .await
    }

    /// Mark `quantity` of an order as traded. The server records a transaction,
    /// lowers the order's quantity and removes the order once it reaches zero.
    pub async fn close_order(&self, id: &str, quantity: u32) -> Result<Transaction> {
        self.authed(
            reqwest::Method::POST,
            &format!("/order/{id}/close"),
            Some(serde_json::json!({ "quantity": quantity })),
        )
        .await
    }

    pub async fn delete_order(&self, id: &str) -> Result<()> {
        let _: serde_json::Value = self
            .authed(reqwest::Method::DELETE, &format!("/order/{id}"), None)
            .await
            .or_else(|e| if e.to_string().contains("empty data") { Ok(serde_json::Value::Null) } else { Err(e) })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_close_order_response() {
        // Shape from docs.warframe.market (Close Order -> compact Transaction).
        let body = r#"{"apiVersion":"0.22.0","data":{"id":"tx1","type":"sell","originId":"ord1","platinum":15,
            "quantity":2,"createdAt":"2026-10-27T08:00:00Z","updatedAt":"2026-10-27T08:00:00Z"},"error":null}"#;
        let env: Envelope<Transaction> = serde_json::from_str(body).unwrap();
        let tx = env.data.unwrap();
        assert_eq!((tx.kind.as_str(), tx.origin_id.as_str(), tx.quantity), ("sell", "ord1", 2));
        assert_eq!(tx.platinum, 15.0);
    }
}
