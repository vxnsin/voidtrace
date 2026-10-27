//! World state for the widgets: open-world cycles, void fissures, Baro, sortie.
//! Comes from api.warframestat.us, the WFCD parse of DE's worldState.php, so node
//! names and mission types are already readable. Polled once a minute; the UI
//! counts the timers down locally.

use anyhow::{Context, Result};
use serde_json::{Map, Value};

const URL: &str = "https://api.warframestat.us/pc/?language=en";
const KEYS: &[&str] = &[
    "timestamp", "cetusCycle", "vallisCycle", "cambionCycle", "zarimanCycle", "duviriCycle", "earthCycle",
    "fissures", "voidTrader", "sortie", "archonHunt", "arbitration", "steelPath",
];

/// Only the parts the widgets use; the full document is ~180 KB.
pub async fn fetch(http: &reqwest::Client) -> Result<Value> {
    let v: Value = http
        .get(URL)
        .header("User-Agent", crate::market::USER_AGENT)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await
        .context("worldstate is not JSON")?;
    let mut out = Map::new();
    for k in KEYS {
        if let Some(x) = v.get(*k) {
            out.insert((*k).to_string(), x.clone());
        }
    }
    Ok(Value::Object(out))
}
