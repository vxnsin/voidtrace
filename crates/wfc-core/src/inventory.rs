//! Turn the raw inventory document into things the app understands.

use crate::export::Export;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

const DUCATS: &str = "/Lotus/Types/Items/MiscItems/PrimeBucks";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwnedItem {
    pub path: String,
    pub name: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Snapshot {
    pub platinum: i64,
    pub credits: i64,
    pub ducats: i64,
    pub items: Vec<OwnedItem>,
}

fn counted<'a>(v: &'a Value, key: &str) -> impl Iterator<Item = (String, i64)> + 'a {
    v.get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|e| {
            let t = e.get("ItemType")?.as_str()?.to_string();
            let n = e.get("ItemCount").and_then(Value::as_i64).unwrap_or(1);
            Some((t, n))
        })
}

pub fn parse(ex: &Export, v: &Value) -> Snapshot {
    let mut counts: HashMap<String, i64> = HashMap::new();
    // MiscItems: prime parts, relics, resources. Recipes: blueprints.
    // RawUpgrades: unranked mod stacks. Upgrades: individual (ranked) mods.
    for key in ["MiscItems", "Recipes", "RawUpgrades", "Upgrades"] {
        for (t, n) in counted(v, key) {
            // Rivens are unique objects; they are not stackable or priceable here.
            if t.contains("/Randomized/") {
                continue;
            }
            *counts.entry(t).or_default() += n;
        }
    }
    let ducats = counts.remove(DUCATS).unwrap_or(0);
    let mut items: Vec<OwnedItem> = counts
        .into_iter()
        .filter(|(_, n)| *n > 0)
        .map(|(path, count)| OwnedItem { name: ex.item_name(&path), path, count })
        .collect();
    items.sort_by(|a, b| a.name.cmp(&b.name));
    Snapshot {
        platinum: v.get("PremiumCredits").and_then(Value::as_i64).unwrap_or(0),
        credits: v.get("RegularCredits").and_then(Value::as_i64).unwrap_or(0),
        ducats,
        items,
    }
}
