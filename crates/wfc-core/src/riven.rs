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
