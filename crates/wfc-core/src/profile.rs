//! DE profile endpoint. Gives XP per item for a player, which is everything
//! mastery and foundry need.
//!
//! Since U38.0.8 the lookup needs the account id (24 hex chars), not the name.
//! Invalid ids or hammering the endpoint can get an IP temporarily blocked by
//! DE, so this is only called on explicit user action and the result is cached.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

const URL: &str = "https://api.warframe.com/cdn/getProfileViewingData.php";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XpEntry {
    #[serde(rename = "ItemType")]
    pub item_type: String,
    #[serde(rename = "XP")]
    pub xp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Profile {
    pub display_name: String,
    pub player_level: u32,
    pub xp: Vec<XpEntry>,
}

pub fn valid_id(id: &str) -> bool {
    id.len() == 24 && id.chars().all(|c| c.is_ascii_hexdigit())
}

pub async fn fetch(client: &reqwest::Client, player_id: &str) -> Result<Profile> {
    if !valid_id(player_id) {
        bail!("account id must be 24 hex characters");
    }
    let res = client.get(URL).query(&[("playerId", player_id)]).send().await?;
    let status = res.status();
    let text = res.text().await?;
    if !status.is_success() {
        bail!("profile request failed: {status}");
    }
    if text.trim().is_empty() {
        bail!("empty profile response: unknown id for this platform, or DE is rate-limiting your network");
    }
    let v: serde_json::Value = serde_json::from_str(&text).context("profile is not JSON")?;
    let r = v
        .get("Results")
        .and_then(|r| r.get(0))
        .unwrap_or(&v);
    let display_name = r["DisplayName"].as_str().unwrap_or("").to_string();
    let player_level = r["PlayerLevel"].as_u64().unwrap_or(0) as u32;
    let xp = r
        .pointer("/LoadOutInventory/XPInfo")
        .and_then(|x| serde_json::from_value(x.clone()).ok())
        .unwrap_or_default();
    Ok(Profile { display_name, player_level, xp })
}
