//! Foundry, sets and mastery routes: DE's recipes crossed with what the
//! inventory sync saw (every item path with its count).

use crate::export::{Export, ExportItem, Recipe};
use crate::market::MarketItem;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Item path -> count, from the last inventory sync.
pub type Owned = HashMap<String, i64>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Part {
    pub path: String,
    pub name: String,
    pub need: i64,
    pub have: i64,
    /// The part has its own blueprint (a component); `bp_have` counts those.
    pub blueprint: Option<String>,
    pub bp_have: i64,
    /// Tradable on warframe.market (the part itself or its blueprint).
    pub slug: Option<String>,
    pub vaulted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Build {
    pub unique_name: String,
    pub name: String,
    pub category: String,
    pub image: Option<String>,
    pub mastery_req: u32,
    /// Main blueprint path and how many I have.
    pub blueprint: Option<String>,
    pub bp_have: i64,
    pub bp_slug: Option<String>,
    pub parts: Vec<Part>,
    pub credits: i64,
    /// Blueprint and every part (built, or buildable from an owned blueprint) at hand.
    pub ready: bool,
    pub vaulted: bool,
}

pub struct Index<'a> {
    by_result: HashMap<&'a str, &'a Recipe>,
    market: HashMap<&'a str, &'a MarketItem>,
}

impl<'a> Index<'a> {
    pub fn new(ex: &'a Export, items: &'a [MarketItem]) -> Self {
        Index {
            by_result: ex.recipes.iter().map(|r| (r.result_type.as_str(), r)).collect(),
            market: items.iter().filter(|i| !i.game_ref.is_empty()).map(|i| (i.game_ref.as_str(), i)).collect(),
        }
    }
    pub fn recipe_for(&self, result: &str) -> Option<&'a Recipe> {
        self.by_result.get(result).copied()
    }
    pub fn market(&self, path: &str) -> Option<&'a MarketItem> {
        self.market.get(path).copied()
    }
}

pub fn is_resource(path: &str) -> bool {
    path.contains("/Types/Items/")
}

fn count(owned: &Owned, path: &str) -> i64 {
    owned.get(path).copied().unwrap_or(0)
}

pub fn build(ex: &Export, idx: &Index, owned: &Owned, it: &ExportItem) -> Build {
    let recipe = idx.recipe_for(&it.unique_name);
    let mut parts = vec![];
    if let Some(r) = recipe {
        for ing in &r.ingredients {
            // Resources (Orokin Cells, Neural Sensors) have recipes too, but they
            // are farmed, not parts with blueprints.
            let sub = if is_resource(&ing.item_type) { None } else { idx.recipe_for(&ing.item_type) };
            let bp = sub.map(|s| s.unique_name.clone());
            // The tradable thing is the part itself (weapon parts) or its blueprint.
            let m = idx.market(&ing.item_type).or_else(|| bp.as_deref().and_then(|b| idx.market(b)));
            parts.push(Part {
                name: ex.item_name(&ing.item_type),
                path: ing.item_type.clone(),
                need: ing.count,
                have: count(owned, &ing.item_type),
                bp_have: bp.as_deref().map(|b| count(owned, b)).unwrap_or(0),
                blueprint: bp,
                slug: m.map(|m| m.slug.clone()),
                vaulted: m.is_some_and(|m| m.vaulted),
            });
        }
    }
    let bp = recipe.map(|r| r.unique_name.clone());
    let bp_have = bp.as_deref().map(|b| count(owned, b)).unwrap_or(0);
    let ready = recipe.is_some()
        && bp_have > 0
        && parts.iter().all(|p| p.have + if p.blueprint.is_some() { p.bp_have } else { 0 } >= p.need);
    let bp_market = bp.as_deref().and_then(|b| idx.market(b));
    Build {
        unique_name: it.unique_name.clone(),
        name: it.name.clone(),
        category: it.product_category.clone(),
        image: ex.images.get(&it.unique_name).cloned(),
        mastery_req: it.mastery_req,
        vaulted: bp_market.is_some_and(|m| m.vaulted) || parts.iter().any(|p| p.vaulted),
        bp_slug: bp_market.map(|m| m.slug.clone()),
        blueprint: bp,
        bp_have,
        credits: recipe.map(|r| r.build_price).unwrap_or(0),
        parts,
        ready,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetPart {
    pub slug: String,
    pub name: String,
    pub need: i64,
    pub have: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetInfo {
    pub slug: String,
    pub name: String,
    pub parts: Vec<SetPart>,
    /// Full sets I could hand over right now.
    pub complete: i64,
}

/// warframe.market sets whose tradable parts I own at least one of. Parts come
/// from DE's recipe of the set's item: the main blueprint plus every component
/// (its blueprint when the component itself is not tradable).
pub fn sets(idx: &Index, items: &[MarketItem], have_by_slug: &HashMap<String, i64>) -> Vec<SetInfo> {
    let mut out = vec![];
    for set in items.iter().filter(|i| i.tags.iter().any(|t| t == "set") && !i.game_ref.is_empty()) {
        let Some(r) = idx.recipe_for(&set.game_ref) else { continue };
        let mut parts: Vec<SetPart> = vec![];
        let mut add = |m: &MarketItem, need: i64| {
            if let Some(p) = parts.iter_mut().find(|p| p.slug == m.slug) {
                p.need += need;
            } else {
                parts.push(SetPart { slug: m.slug.clone(), name: m.name.clone(), need, have: have_by_slug.get(&m.slug).copied().unwrap_or(0) });
            }
        };
        if let Some(m) = idx.market(&r.unique_name) {
            add(m, 1);
        }
        for ing in &r.ingredients {
            let m = idx.market(&ing.item_type).or_else(|| idx.recipe_for(&ing.item_type).and_then(|s| idx.market(&s.unique_name)));
            if let Some(m) = m {
                add(m, ing.count);
            }
        }
        if parts.len() < 2 || parts.iter().all(|p| p.have == 0) {
            continue;
        }
        let complete = parts.iter().map(|p| p.have / p.need.max(1)).min().unwrap_or(0);
        out.push(SetInfo { slug: set.slug.clone(), name: set.name.clone(), parts, complete });
    }
    out.sort_by(|a, b| {
        b.complete
            .cmp(&a.complete)
            .then_with(|| owned_share(b).total_cmp(&owned_share(a)))
            .then_with(|| a.name.cmp(&b.name))
    });
    out
}

fn owned_share(s: &SetInfo) -> f64 {
    s.parts.iter().filter(|p| p.have >= p.need).count() as f64 / s.parts.len().max(1) as f64
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutePart {
    pub name: String,
    /// Chance to get it from the relics I own, cracking every copy at its refinement.
    pub chance: f64,
    pub have: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelicRoute {
    pub unique_name: String,
    pub name: String,
    pub parts: Vec<RoutePart>,
    /// All missing parts before the relics run out (parts independent).
    pub chance_all: f64,
}

/// Drop chance per rarity per refinement, as in relic.rs (0..1).
fn chance(rarity: &str, refine: usize) -> f64 {
    let t: [f64; 4] = match rarity {
        "RARE" => [2.0, 4.0, 6.0, 10.0],
        "UNCOMMON" => [11.0, 13.0, 17.0, 20.0],
        _ => [25.33, 23.33, 20.0, 16.67],
    };
    t[refine] / 100.0
}

/// Unmastered items I could finish from relics I own: for every part I lack,
/// the chance that at least one owned relic copy drops it (solo cracks).
pub fn relic_routes(
    ex: &Export,
    idx: &Index,
    owned: &Owned,
    relics: &HashMap<String, [i64; 4]>,
    todo: &[&ExportItem],
) -> Vec<RelicRoute> {
    // Reward name -> (relic name, rarity), for relics I own.
    let mut drops: HashMap<String, Vec<(&str, &str)>> = HashMap::new();
    for r in &ex.relics {
        if relics.get(&r.name).map(|c| c.iter().sum::<i64>()).unwrap_or(0) == 0 {
            continue;
        }
        for d in &r.relic_rewards {
            drops.entry(crate::norm(&ex.reward_name(&d.reward_name))).or_default().push((r.name.as_str(), d.rarity.as_str()));
        }
    }
    let mut out = vec![];
    for it in todo {
        let b = build(ex, idx, owned, it);
        let Some(bp) = &b.blueprint else { continue };
        // Main blueprint and parts, by the names relics use ("… Blueprint").
        let mut wanted: Vec<(String, bool)> = vec![(ex.item_name(bp), b.bp_have > 0)];
        for p in b.parts.iter().filter(|p| !is_resource(&p.path)) {
            let name = p.blueprint.as_deref().map(|x| ex.item_name(x)).unwrap_or_else(|| p.name.clone());
            wanted.push((name, p.have + p.bp_have >= p.need));
        }
        let missing: Vec<&(String, bool)> = wanted.iter().filter(|(_, h)| !h).collect();
        if missing.is_empty() || !missing.iter().all(|(n, _)| drops.contains_key(&crate::norm(n))) {
            continue;
        }
        let mut parts = vec![];
        let mut all = 1.0;
        for (name, have) in &wanted {
            let c = if *have {
                1.0
            } else {
                let mut miss = 1.0;
                for (relic, rarity) in drops.get(&crate::norm(name)).into_iter().flatten() {
                    let counts = relics.get(*relic).copied().unwrap_or_default();
                    for (refine, n) in counts.iter().enumerate() {
                        miss *= (1.0 - chance(rarity, refine)).powi(*n as i32);
                    }
                }
                1.0 - miss
            };
            all *= c;
            parts.push(RoutePart { name: name.clone(), chance: c, have: *have });
        }
        out.push(RelicRoute { unique_name: it.unique_name.clone(), name: it.name.clone(), parts, chance_all: all });
    }
    out.sort_by(|a, b| b.chance_all.total_cmp(&a.chance_all));
    out
}

/// Railjack and Drifter intrinsics from the inventory's `PlayerSkills`; every
/// rank is worth 1500 mastery, ten ranks per skill.
pub const INTRINSICS: &[(&str, &str)] = &[
    ("LPS_TACTICAL", "Tactical"),
    ("LPS_PILOTING", "Piloting"),
    ("LPS_GUNNERY", "Gunnery"),
    ("LPS_ENGINEERING", "Engineering"),
    ("LPS_COMMAND", "Command"),
    ("LPS_DRIFT_COMBAT", "Drifter combat"),
    ("LPS_DRIFT_RIDING", "Drifter riding"),
    ("LPS_DRIFT_OPPORTUNITY", "Drifter opportunity"),
    ("LPS_DRIFT_ENDURANCE", "Drifter endurance"),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::Ingredient;

    fn mi(slug: &str, game_ref: &str, tags: &[&str]) -> MarketItem {
        MarketItem {
            id: slug.into(),
            slug: slug.into(),
            name: slug.replace('_', " "),
            icon: String::new(),
            ducats: 0,
            tradable: true,
            tags: tags.iter().map(|t| t.to_string()).collect(),
            vaulted: false,
            game_ref: game_ref.into(),
        }
    }

    #[test]
    fn set_parts_from_recipes() {
        let ex = Export {
            recipes: vec![Recipe {
                unique_name: "/BP/Kamas".into(),
                result_type: "/W/Kamas".into(),
                build_price: 0,
                ingredients: vec![
                    Ingredient { item_type: "/C/Blade".into(), count: 2 },
                    Ingredient { item_type: "/C/Handle".into(), count: 1 },
                ],
            }],
            ..Default::default()
        };
        let items = vec![
            mi("kamas_set", "/W/Kamas", &["set"]),
            mi("kamas_blueprint", "/BP/Kamas", &[]),
            mi("kamas_blade", "/C/Blade", &[]),
            mi("kamas_handle", "/C/Handle", &[]),
        ];
        let idx = Index::new(&ex, &items);
        let have: HashMap<String, i64> =
            [("kamas_blade".to_string(), 3), ("kamas_handle".to_string(), 1), ("kamas_blueprint".to_string(), 1)].into();
        let s = sets(&idx, &items, &have);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].parts.iter().find(|p| p.slug == "kamas_blade").unwrap().need, 2);
        assert_eq!(s[0].complete, 1);
    }
}
