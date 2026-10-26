//! Relic planner: which relic to crack next for a wanted item, and what a
//! relic is worth in plat.

use crate::export::{Export, ExportRelic};
use crate::market::Market;
use serde::{Deserialize, Serialize};

/// Drop chance per rarity per refinement (Intact, Exceptional, Flawless, Radiant).
/// Per-slot chance: common slots share 3 ways, uncommon 2, rare 1.
const COMMON: [f32; 4] = [25.33, 23.33, 20.0, 16.67];
const UNCOMMON: [f32; 4] = [11.0, 13.0, 17.0, 20.0];
const RARE: [f32; 4] = [2.0, 4.0, 6.0, 10.0];

pub const REFINEMENTS: [&str; 4] = ["Intact", "Exceptional", "Flawless", "Radiant"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DropInfo {
    pub item: String,
    pub rarity: String,
    /// chance in % for each refinement
    pub chance: [f32; 4],
    pub plat: Option<u32>,
    pub ducats: u32,
    pub slug: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelicPlan {
    pub relic: String,
    pub drops: Vec<DropInfo>,
    /// Expected plat per crack, by refinement.
    pub expected_plat: [f32; 4],
    pub expected_ducats: [f32; 4],
}

fn chance_for(rarity: &str) -> [f32; 4] {
    match rarity {
        "RARE" => RARE,
        "UNCOMMON" => UNCOMMON,
        _ => COMMON,
    }
}

/// Reward name in export is a /Lotus path; the last segment is usable but the
/// market name is nicer. Map "Forma Blueprint" and "… Prime Blueprint" etc.
pub fn reward_display(path: &str) -> String {
    let seg = path.rsplit('/').next().unwrap_or(path);
    // Insert spaces between camel-case words: "AshPrimeBlueprint" -> "Ash Prime Blueprint"
    let mut out = String::new();
    for (i, c) in seg.chars().enumerate() {
        if i > 0 && c.is_uppercase() {
            out.push(' ');
        }
        out.push(c);
    }
    out.replace("Prime Systems", "Prime Systems Blueprint")
        .replace("Prime Chassis", "Prime Chassis Blueprint")
        .replace("Prime Neuroptics", "Prime Neuroptics Blueprint")
}

pub async fn plan(ex: &Export, market: &Market, relic: &ExportRelic) -> RelicPlan {
    let mut drops = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for r in &relic.relic_rewards {
        let name = ex.reward_name(&r.reward_name);
        if !seen.insert(name.clone()) {
            continue;
        }
        let (plat, ducats, slug) = match market.find_item(&name) {
            Some(mi) => {
                let p = market.price(&mi.slug).await.ok();
                (p.as_ref().and_then(|p| p.sell), mi.ducats, Some(mi.slug))
            }
            None => (None, 0, None),
        };
        drops.push(DropInfo { item: name, rarity: r.rarity.clone(), chance: chance_for(&r.rarity), plat, ducats, slug });
    }
    let mut expected_plat = [0f32; 4];
    let mut expected_ducats = [0f32; 4];
    for d in &drops {
        for i in 0..4 {
            expected_plat[i] += d.chance[i] / 100.0 * d.plat.unwrap_or(0) as f32;
            expected_ducats[i] += d.chance[i] / 100.0 * d.ducats as f32;
        }
    }
    RelicPlan { relic: relic.name.clone(), drops, expected_plat, expected_ducats }
}

/// All relics dropping `wanted` (fuzzy on display name), best chance first.
pub fn relics_for(ex: &Export, wanted: &str) -> Vec<(String, String, [f32; 4])> {
    let w = crate::norm(wanted);
    let mut out: Vec<(String, String, [f32; 4])> = ex
        .relics
        .iter()
        .filter_map(|r| {
            let hit = r
                .relic_rewards
                .iter()
                .find(|d| crate::norm(&ex.reward_name(&d.reward_name)).contains(&w))?;
            Some((r.name.clone(), hit.rarity.clone(), chance_for(&hit.rarity)))
        })
        .collect();
    out.sort_by(|a, b| b.2[3].partial_cmp(&a.2[3]).unwrap());
    out
}
