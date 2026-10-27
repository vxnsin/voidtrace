//! Riven grading. Base values are the community-documented per-stat maxima at
//! disposition 1.0; actual range = base * disposition * curse/buff multiplier.

use crate::export::Export;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WeaponClass {
    Rifle,
    Shotgun,
    Pistol,
    Melee,
    Archgun,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatInput {
    pub stat: String,
    pub value: f32,
    pub negative: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatGrade {
    pub stat: String,
    pub value: f32,
    pub min: f32,
    pub max: f32,
    pub grade: String,
    pub negative: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RivenGrade {
    pub weapon: String,
    pub disposition: f32,
    pub stats: Vec<StatGrade>,
}

/// (stat key, rifle, shotgun, pistol, melee, archgun) max at dispo 1.0, 2 positives 0 negatives... we
/// store the base and apply the buff multiplier separately.
const BASE: &[(&str, [f32; 5])] = &[
    ("damage", [16.5, 16.5, 22.0, 16.5, 10.0]),
    ("multishot", [9.0, 12.0, 12.0, 0.0, 6.0]),
    ("critical_chance", [15.0, 9.0, 15.0, 18.0, 10.0]),
    ("critical_damage", [12.0, 9.0, 9.0, 9.0, 8.0]),
    ("status_chance", [9.0, 9.0, 9.0, 9.0, 6.0]),
    ("status_duration", [10.0, 10.0, 10.0, 10.0, 10.0]),
    ("fire_rate", [6.0, 9.0, 7.5, 5.5, 6.0]),
    ("heat", [9.0, 9.0, 9.0, 9.0, 12.0]),
    ("cold", [9.0, 9.0, 9.0, 9.0, 12.0]),
    ("electricity", [9.0, 9.0, 9.0, 9.0, 12.0]),
    ("toxin", [9.0, 9.0, 9.0, 9.0, 12.0]),
    ("impact", [12.0, 12.0, 12.0, 12.0, 12.0]),
    ("puncture", [12.0, 12.0, 12.0, 12.0, 12.0]),
    ("slash", [12.0, 12.0, 12.0, 12.0, 12.0]),
    ("magazine", [5.0, 5.0, 5.0, 0.0, 6.0]),
    ("ammo_max", [5.0, 9.0, 9.0, 0.0, 10.0]),
    ("reload_speed", [5.0, 5.0, 5.0, 0.0, 10.0]),
    ("zoom", [6.0, 12.0, 8.0, 0.0, 0.0]),
    ("projectile_speed", [9.0, 9.0, 9.0, 0.0, 9.0]),
    ("recoil", [9.0, 9.0, 9.0, 0.0, 0.0]),
    ("punch_through", [2.7, 2.7, 2.7, 0.0, 2.7]),
    ("grineer", [4.5, 4.5, 4.5, 4.5, 4.5]),
    ("corpus", [4.5, 4.5, 4.5, 4.5, 4.5]),
    ("infested", [4.5, 4.5, 4.5, 4.5, 4.5]),
    ("range", [0.0, 0.0, 0.0, 12.0, 0.0]),
    ("combo_duration", [0.0, 0.0, 0.0, 8.0, 0.0]),
    ("slide_crit", [0.0, 0.0, 0.0, 12.0, 0.0]),
    ("finisher_damage", [0.0, 0.0, 0.0, 12.0, 0.0]),
    ("initial_combo", [0.0, 0.0, 0.0, 25.0, 0.0]),
    ("heavy_efficiency", [0.0, 0.0, 0.0, 7.3, 0.0]),
    ("combo_chance", [0.0, 0.0, 0.0, 9.0, 0.0]),
];

pub fn classify(ex: &Export, weapon: &str) -> (f32, WeaponClass) {
    let w = crate::norm(weapon);
    let found = ex.weapons.iter().find(|i| crate::norm(&i.name) == w);
    let dispo = found.map(|i| i.disposition).filter(|d| *d > 0.0).unwrap_or(1.0);
    let class = match found.map(|i| i.product_category.as_str()) {
        Some("Melee") => WeaponClass::Melee,
        Some("Pistols") => WeaponClass::Pistol,
        Some("SpaceGuns") => WeaponClass::Archgun,
        Some("LongGuns") => {
            let u = found.map(|i| i.unique_name.to_ascii_lowercase()).unwrap_or_default();
            if u.contains("shotgun") { WeaponClass::Shotgun } else { WeaponClass::Rifle }
        }
        _ => WeaponClass::Rifle,
    };
    (dispo, class)
}

/// Buff multiplier by (positives, has negative). Curse multiplier for the negative.
fn multipliers(pos: usize, neg: bool) -> (f32, f32) {
    match (pos, neg) {
        (2, false) => (0.99, 0.0),
        (2, true) => (1.2375, 0.495),
        (3, false) => (0.75, 0.0),
        (3, true) => (0.9375, 0.75),
        _ => (1.0, 0.75),
    }
}

pub fn grade(ex: &Export, weapon: &str, stats: &[StatInput]) -> RivenGrade {
    let (dispo, class) = classify(ex, weapon);
    let col = match class {
        WeaponClass::Rifle => 0,
        WeaponClass::Shotgun => 1,
        WeaponClass::Pistol => 2,
        WeaponClass::Melee => 3,
        WeaponClass::Archgun => 4,
    };
    let pos = stats.iter().filter(|s| !s.negative).count();
    let neg = stats.iter().any(|s| s.negative);
    let (buff, curse) = multipliers(pos, neg);
    let graded = stats
        .iter()
        .map(|s| {
            let base = BASE.iter().find(|(k, _)| *k == s.stat).map(|(_, v)| v[col]).unwrap_or(0.0);
            let mult = if s.negative { curse } else { buff };
            let center = base * dispo * mult;
            // Base values are unranked; rank 8 shows 10x. Detect which the user typed.
            let value = if center > 0.0 && s.value.abs() > center * 4.0 { s.value / 10.0 } else { s.value };
            // Riven roll range is ±10% of the center.
            let (min, max) = (center * 0.9, center * 1.1);
            let rel = if center > 0.0 { (value.abs() - center) / center } else { 0.0 };
            // Negatives are "better" when smaller in magnitude.
            let rel = if s.negative { -rel } else { rel };
            let grade = match rel {
                r if r >= 0.09 => "S",
                r if r >= 0.07 => "A+",
                r if r >= 0.05 => "A",
                r if r >= 0.03 => "A-",
                r if r >= 0.01 => "B+",
                r if r >= -0.01 => "B",
                r if r >= -0.03 => "B-",
                r if r >= -0.05 => "C+",
                r if r >= -0.07 => "C",
                r if r >= -0.09 => "C-",
                _ => "F",
            };
            StatGrade { stat: s.stat.clone(), value, min, max, grade: grade.into(), negative: s.negative }
        })
        .collect();
    RivenGrade { weapon: weapon.to_string(), disposition: dispo, stats: graded }
}

// ---- Riven market: warframe.market auctions and DE weekly trade data ----

use anyhow::Context;
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RivenWeapon {
    pub slug: String,
    pub name: String,
    pub riven_type: String,
    pub disposition: f32,
    pub icon: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RivenAttr {
    pub url_name: String,
    pub value: f32,
    pub positive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RivenAuction {
    pub id: String,
    pub name: String,
    pub buyout: Option<u32>,
    pub starting: u32,
    pub top_bid: Option<u32>,
    pub direct: bool,
    pub rerolls: u32,
    pub mastery: u32,
    pub polarity: String,
    pub attributes: Vec<RivenAttr>,
    pub owner: String,
    pub owner_status: String,
    pub created: String,
}

/// One row of DE's weekly riven trade statistics (www-static.warframe.com).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeeklyRiven {
    #[serde(rename = "itemType")]
    pub item_type: String,
    #[serde(default)]
    pub compatibility: Option<String>,
    #[serde(default)]
    pub rerolled: bool,
    #[serde(default)]
    pub avg: f32,
    #[serde(default)]
    pub stddev: f32,
    #[serde(default)]
    pub min: f32,
    #[serde(default)]
    pub max: f32,
    #[serde(default)]
    pub pop: f32,
    #[serde(default)]
    pub median: f32,
}

fn wfm_get(http: &reqwest::Client, url: &str) -> reqwest::RequestBuilder {
    http.get(url)
        .header("Platform", "pc")
        .header("Language", "en")
        .header("Crossplay", "true")
        .header("User-Agent", crate::market::USER_AGENT)
}

pub async fn riven_weapons(http: &reqwest::Client) -> anyhow::Result<Vec<RivenWeapon>> {
    let v: Value = wfm_get(http, "https://api.warframe.market/v2/riven/weapons").send().await?.error_for_status()?.json().await?;
    let list = v["data"].as_array().context("no riven weapons")?;
    let mut out: Vec<RivenWeapon> = list
        .iter()
        .filter_map(|w| {
            Some(RivenWeapon {
                slug: w["slug"].as_str()?.to_string(),
                name: w.pointer("/i18n/en/name")?.as_str()?.to_string(),
                riven_type: w["rivenType"].as_str().unwrap_or("").to_string(),
                disposition: w["disposition"].as_f64().unwrap_or(1.0) as f32,
                icon: w.pointer("/i18n/en/icon").and_then(Value::as_str).unwrap_or("").to_string(),
            })
        })
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// Direct-sell (buyout) riven auctions for a weapon, cheapest first. This is
/// the contract search the site uses (v1); keep calls rare.
pub async fn riven_auctions(http: &reqwest::Client, weapon: &str) -> anyhow::Result<Vec<RivenAuction>> {
    let url = format!(
        "https://api.warframe.market/v1/auctions/search?type=riven&weapon_url_name={weapon}&sort_by=price_asc&buyout_policy=direct"
    );
    let v: Value = wfm_get(http, &url).send().await?.error_for_status()?.json().await?;
    let list = v.pointer("/payload/auctions").and_then(Value::as_array).context("no auctions in answer")?;
    Ok(list
        .iter()
        .filter(|a| a["visible"].as_bool() != Some(false) && a["closed"].as_bool() != Some(true))
        .filter_map(|a| {
            let it = &a["item"];
            Some(RivenAuction {
                id: a["id"].as_str()?.to_string(),
                name: it["name"].as_str().unwrap_or("").to_string(),
                buyout: a["buyout_price"].as_u64().map(|x| x as u32),
                starting: a["starting_price"].as_u64().unwrap_or(0) as u32,
                top_bid: a["top_bid"].as_u64().map(|x| x as u32),
                direct: a["is_direct_sell"].as_bool().unwrap_or(false),
                rerolls: it["re_rolls"].as_u64().unwrap_or(0) as u32,
                mastery: it["mastery_level"].as_u64().unwrap_or(0) as u32,
                polarity: it["polarity"].as_str().unwrap_or("").to_string(),
                attributes: it["attributes"]
                    .as_array()
                    .map(|x| {
                        x.iter()
                            .filter_map(|t| {
                                Some(RivenAttr {
                                    url_name: t["url_name"].as_str()?.to_string(),
                                    value: t["value"].as_f64().unwrap_or(0.0) as f32,
                                    positive: t["positive"].as_bool().unwrap_or(true),
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
                owner: a.pointer("/owner/ingame_name").and_then(Value::as_str).unwrap_or("").to_string(),
                owner_status: a.pointer("/owner/status").and_then(Value::as_str).unwrap_or("").to_string(),
                created: a["created"].as_str().unwrap_or("").to_string(),
            })
        })
        .collect())
}

pub async fn weekly_rivens(http: &reqwest::Client) -> anyhow::Result<Vec<WeeklyRiven>> {
    let text = http.get("https://www-static.warframe.com/repos/weeklyRivensPC.json").send().await?.error_for_status()?.text().await?;
    Ok(serde_json::from_str(&js_to_json(&text))?)
}

/// DE ships this file as a JavaScript literal (bare keys, single quotes).
fn js_to_json(s: &str) -> String {
    let keys = regex::Regex::new(r"(?m)^(\s*)([A-Za-z_][A-Za-z0-9_]*)\s*:").unwrap();
    let quoted = regex::Regex::new(r"'([^']*)'").unwrap();
    let s = keys.replace_all(s, "$1\"$2\":");
    quoted.replace_all(&s, |c: &regex::Captures| serde_json::to_string(&c[1]).unwrap_or_default()).into_owned()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttrPrice {
    pub url_name: String,
    pub count: usize,
    pub median: f32,
    /// Median of auctions with this positive stat over the median of all.
    pub ratio: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RivenPrices {
    pub count: usize,
    pub median: f32,
    /// Mean after dropping outliers (outside 1.5 IQR).
    pub trimmed_avg: f32,
    pub low: f32,
    pub high: f32,
    pub attrs: Vec<AttrPrice>,
}

fn median(v: &mut [f32]) -> f32 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.total_cmp(b));
    let m = v.len() / 2;
    if v.len() % 2 == 0 { (v[m - 1] + v[m]) / 2.0 } else { v[m] }
}

/// Price picture from current buyout auctions: the middle of the market
/// without outliers, and which positive stats come with higher prices.
pub fn riven_prices(auctions: &[RivenAuction]) -> RivenPrices {
    let mut prices: Vec<f32> = auctions.iter().filter_map(|a| a.buyout).map(|p| p as f32).collect();
    if prices.is_empty() {
        return RivenPrices::default();
    }
    prices.sort_by(|a, b| a.total_cmp(b));
    let q = |f: f32| prices[((prices.len() - 1) as f32 * f).round() as usize];
    let (q1, q3) = (q(0.25), q(0.75));
    let (lo, hi) = (q1 - 1.5 * (q3 - q1), q3 + 1.5 * (q3 - q1));
    let kept: Vec<f32> = prices.iter().copied().filter(|p| *p >= lo && *p <= hi).collect();
    let all_median = median(&mut prices.clone());
    let mut by_attr: std::collections::HashMap<&str, Vec<f32>> = Default::default();
    for a in auctions {
        let Some(p) = a.buyout else { continue };
        for t in a.attributes.iter().filter(|t| t.positive) {
            by_attr.entry(t.url_name.as_str()).or_default().push(p as f32);
        }
    }
    let mut attrs: Vec<AttrPrice> = by_attr
        .into_iter()
        .filter(|(_, v)| v.len() >= 3)
        .map(|(k, mut v)| {
            let m = median(&mut v);
            AttrPrice { url_name: k.to_string(), count: v.len(), median: m, ratio: if all_median > 0.0 { m / all_median } else { 1.0 } }
        })
        .collect();
    attrs.sort_by(|a, b| b.ratio.total_cmp(&a.ratio));
    RivenPrices {
        count: prices.len(),
        median: all_median,
        trimmed_avg: kept.iter().sum::<f32>() / kept.len().max(1) as f32,
        low: kept.first().copied().unwrap_or(0.0),
        high: kept.last().copied().unwrap_or(0.0),
        attrs,
    }
}

#[cfg(test)]
mod market_tests {
    use super::*;

    fn auc(p: u32, attrs: &[&str]) -> RivenAuction {
        RivenAuction {
            id: p.to_string(),
            name: String::new(),
            buyout: Some(p),
            starting: p,
            top_bid: None,
            direct: true,
            rerolls: 0,
            mastery: 8,
            polarity: String::new(),
            owner: String::new(),
            owner_status: String::new(),
            created: String::new(),
            attributes: attrs.iter().map(|a| RivenAttr { url_name: a.to_string(), value: 1.0, positive: true }).collect(),
        }
    }

    #[test]
    fn weekly_file_parses() {
        let raw = "[
  {
    itemType: 'Rifle Riven Mod',
    compatibility: 'Rubico',
    rerolled: false,
    avg: 133.36,
    median: 25
  }
]";
        let w: Vec<WeeklyRiven> = serde_json::from_str(&js_to_json(raw)).unwrap();
        assert_eq!(w[0].compatibility.as_deref(), Some("Rubico"));
        assert_eq!(w[0].median, 25.0);
    }

    #[test]
    fn outliers_do_not_move_the_average() {
        let mut a: Vec<RivenAuction> = (0..9).map(|i| auc(100 + i * 5, &["multishot"])).collect();
        a.push(auc(20000, &["zoom"]));
        let p = riven_prices(&a);
        assert!(p.trimmed_avg < 200.0, "{}", p.trimmed_avg);
        assert_eq!(p.count, 10);
    }
}