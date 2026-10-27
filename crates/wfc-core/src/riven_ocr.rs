//! Riven card on screen -> structured riven. The mod station / cycling screen
//! prints the riven name ("Akbolto Conci-lexicon") and up to four stat lines
//! ("+3.2 Punch Through", "x1.53 Damage to Grineer", "-78.6% Zoom"); Windows
//! OCR gives those back as lines, this turns them into weapon + stats.

use crate::export::Export;
use crate::riven::{RivenAuction, StatInput, WeaponClass};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RivenCard {
    pub weapon: String,
    /// warframe.market weapon slug ("akbolto"), when the weapon is known there.
    pub weapon_slug: Option<String>,
    pub name: String,
    pub stats: Vec<StatInput>,
}

/// (our key, OCR text contains all of these words, market url_name)
const STATS: &[(&str, &[&str], &str)] = &[
    ("grineer", &["grineer"], "damage_vs_grineer"),
    ("corpus", &["corpus"], "damage_vs_corpus"),
    ("infested", &["infested"], "damage_vs_infested"),
    ("slide_crit", &["slide", "critical"], "critical_chance_for_slide_attack"),
    ("critical_chance", &["critical", "chance"], "critical_chance"),
    ("critical_damage", &["critical", "damage"], "critical_damage"),
    ("status_chance", &["status", "chance"], "status_chance"),
    ("status_duration", &["status", "duration"], "status_duration"),
    ("finisher_damage", &["finisher"], "finisher_damage"),
    ("multishot", &["multishot"], "multishot"),
    ("fire_rate", &["fire", "rate"], "fire_rate_/_attack_speed"),
    ("fire_rate", &["attack", "speed"], "fire_rate_/_attack_speed"),
    ("punch_through", &["punch"], "punch_through"),
    ("projectile_speed", &["projectile"], "projectile_speed"),
    ("projectile_speed", &["flight", "speed"], "projectile_speed"),
    ("zoom", &["zoom"], "zoom"),
    ("recoil", &["recoil"], "recoil"),
    ("magazine", &["magazine"], "magazine_capacity"),
    ("ammo_max", &["ammo"], "ammo_maximum"),
    ("reload_speed", &["reload"], "reload_speed"),
    ("combo_duration", &["combo", "duration"], "combo_duration"),
    ("combo_chance", &["combo", "chance"], "chance_to_gain_combo_count"),
    ("combo_chance", &["combo", "count"], "chance_to_gain_combo_count"),
    ("initial_combo", &["initial", "combo"], "initial_combo"),
    ("heavy_efficiency", &["heavy", "efficiency"], "heavy_attack_efficiency"),
    ("range", &["range"], "range"),
    ("heat", &["heat"], "heat_damage"),
    ("cold", &["cold"], "cold_damage"),
    ("electricity", &["electric"], "electric_damage"),
    ("toxin", &["toxin"], "toxin_damage"),
    ("impact", &["impact"], "impact_damage"),
    ("puncture", &["puncture"], "puncture_damage"),
    ("slash", &["slash"], "slash_damage"),
    ("damage", &["damage"], "base_damage_/_melee_damage"),
];

/// Market attribute url_name for one of our stat keys.
pub fn market_attr(key: &str) -> Option<&'static str> {
    STATS.iter().find(|(k, _, _)| *k == key).map(|(_, _, u)| *u)
}

fn stat_key(text: &str) -> Option<&'static str> {
    let t = text.to_ascii_lowercase();
    STATS.iter().find(|(_, words, _)| words.iter().all(|w| t.contains(w))).map(|(k, _, _)| *k)
}

/// "+3,2 Punch Through" / "x1.53 Damage to Grineer" / "-78.6% Zoom" -> stat.
/// OCR likes to turn the leading sign into odd glyphs, so anything before the
/// number is read loosely.
fn parse_stat(line: &str) -> Option<StatInput> {
    let l = line.trim();
    let num_start = l.find(|c: char| c.is_ascii_digit())?;
    let prefix = l[..num_start].trim();
    let rest = &l[num_start..];
    let num_end = rest.find(|c: char| !(c.is_ascii_digit() || c == '.' || c == ',')).unwrap_or(rest.len());
    let num: f32 = rest[..num_end].replace(',', ".").parse().ok()?;
    let tail = rest[num_end..].trim_start_matches('%').trim();
    if tail.len() < 3 {
        return None;
    }
    let key = stat_key(tail)?;
    let multiplier = prefix.eq_ignore_ascii_case("x") || prefix.ends_with('x') || prefix.ends_with('X');
    let minus = prefix.contains('-') || prefix.contains('−') || prefix.contains('–');
    let (value, mut negative) = if multiplier {
        // Faction damage is shown as a multiplier: x1.53 = +53 %, x0.7 = -30 %.
        ((num - 1.0) * 100.0, num < 1.0)
    } else {
        (num, minus)
    };
    // Less recoil is the buff; the card shows it with a minus.
    if key == "recoil" {
        negative = !negative;
    }
    Some(StatInput { stat: key.to_string(), value: value.abs() * if negative { -1.0 } else { 1.0 }, negative })
}

/// Find the riven line: "<weapon> <Riven-name>" where the weapon is one we know.
fn parse_title<'a>(lines: &[String], weapons: impl Iterator<Item = (&'a str, Option<&'a str>)> + Clone) -> Option<(String, Option<String>, String)> {
    let mut best: Option<(usize, String, Option<String>, String)> = None;
    for line in lines {
        let l = line.trim();
        let low = l.to_ascii_lowercase();
        for (w, slug) in weapons.clone() {
            let wl = w.to_ascii_lowercase();
            if low.starts_with(&wl) && low.len() > wl.len() + 2 {
                let rest = l[wl.len()..].trim();
                // Riven names are one or two words, often hyphenated: "Conci-lexicon".
                let words = rest.split_whitespace().count();
                if (1..=2).contains(&words) && best.as_ref().map(|b| b.0 < wl.len()).unwrap_or(true) {
                    best = Some((wl.len(), w.to_string(), slug.map(str::to_string), rest.to_string()));
                }
            }
        }
    }
    best.map(|(_, w, s, n)| (w, s, n))
}

/// Parse OCR lines. `weapons` are (name, market slug) pairs; the export weapon
/// list is the fallback when the market list is empty.
pub fn parse(ex: &Export, weapons: &[(String, String)], lines: &[String]) -> Option<RivenCard> {
    let stats: Vec<StatInput> = lines.iter().filter_map(|l| parse_stat(l)).take(4).collect();
    if stats.len() < 2 {
        return None;
    }
    let title = if weapons.is_empty() {
        parse_title(lines, ex.weapons.iter().map(|w| (w.name.as_str(), None)))
    } else {
        parse_title(lines, weapons.iter().map(|(n, s)| (n.as_str(), Some(s.as_str()))))
    };
    let (weapon, weapon_slug, name) = title?;
    Some(RivenCard { weapon, weapon_slug, name, stats })
}

#[derive(Debug, Clone, Serialize)]
pub struct Suggest {
    pub mandatory: Vec<&'static str>,
    pub optional: Vec<&'static str>,
    pub negatives: Vec<&'static str>,
}

/// General guidance per weapon class — what most builds want on a riven and
/// which negatives cost nothing. Not weapon-specific.
pub fn suggest(class: WeaponClass) -> Suggest {
    match class {
        WeaponClass::Melee => Suggest {
            mandatory: vec!["damage", "critical_chance", "critical_damage", "range"],
            optional: vec!["fire_rate", "status_chance", "heat", "toxin", "electricity", "cold"],
            negatives: vec!["finisher_damage", "combo_duration", "initial_combo", "impact", "puncture", "corpus", "infested"],
        },
        WeaponClass::Shotgun => Suggest {
            mandatory: vec!["multishot", "damage", "critical_chance", "critical_damage"],
            optional: vec!["status_chance", "heat", "toxin", "electricity", "cold", "fire_rate"],
            negatives: vec!["zoom", "ammo_max", "projectile_speed", "impact", "puncture", "corpus", "infested"],
        },
        WeaponClass::Pistol => Suggest {
            mandatory: vec!["multishot", "damage", "critical_chance", "critical_damage"],
            optional: vec!["status_chance", "heat", "toxin", "electricity", "cold", "fire_rate"],
            negatives: vec!["zoom", "recoil", "ammo_max", "impact", "puncture", "corpus", "infested"],
        },
        WeaponClass::Archgun => Suggest {
            mandatory: vec!["multishot", "damage", "critical_chance", "critical_damage"],
            optional: vec!["status_chance", "heat", "toxin", "electricity", "cold"],
            negatives: vec!["zoom", "recoil", "ammo_max", "impact", "puncture"],
        },
        WeaponClass::Rifle => Suggest {
            mandatory: vec!["multishot", "damage", "critical_chance", "critical_damage"],
            optional: vec!["status_chance", "heat", "toxin", "electricity", "cold", "fire_rate"],
            negatives: vec!["zoom", "recoil", "ammo_max", "magazine", "impact", "puncture", "corpus", "infested"],
        },
    }
}

/// Auctions sharing at least `min_shared` of the card's positive stats
/// (and none of its negatives as positives), cheapest first.
pub fn similar(card: &RivenCard, auctions: &[RivenAuction], min_shared: usize) -> Vec<RivenAuction> {
    let pos: Vec<&str> = card.stats.iter().filter(|s| !s.negative).filter_map(|s| market_attr(&s.stat)).collect();
    let neg: Vec<&str> = card.stats.iter().filter(|s| s.negative).filter_map(|s| market_attr(&s.stat)).collect();
    let mut out: Vec<(usize, RivenAuction)> = auctions
        .iter()
        .filter(|a| a.buyout.is_some() || a.direct)
        .filter(|a| !a.attributes.iter().any(|t| t.positive && neg.contains(&t.url_name.as_str())))
        .filter_map(|a| {
            let shared = a.attributes.iter().filter(|t| t.positive && pos.contains(&t.url_name.as_str())).count();
            (shared >= min_shared.min(pos.len().max(1))).then(|| (shared, a.clone()))
        })
        .collect();
    out.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.buyout.unwrap_or(a.1.starting).cmp(&b.1.buyout.unwrap_or(b.1.starting))));
    out.into_iter().map(|(_, a)| a).take(6).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stat_lines() {
        let s = parse_stat("+3,2 Punch Through").unwrap();
        assert_eq!((s.stat.as_str(), s.negative), ("punch_through", false));
        let s = parse_stat("x1,53 Damage to Grineer").unwrap();
        assert_eq!(s.stat, "grineer");
        assert!((s.value - 53.0).abs() < 0.01 && !s.negative);
        let s = parse_stat("+108,9% Projectile Speed").unwrap();
        assert_eq!(s.stat, "projectile_speed");
        let s = parse_stat("-78,6% Zoom").unwrap();
        assert!(s.negative && s.stat == "zoom");
        let s = parse_stat("-30% Weapon Recoil").unwrap();
        assert!(!s.negative && s.stat == "recoil");
        assert!(parse_stat("MR 13").is_none());
        assert!(parse_stat("CYCLE FOR 1.000").is_none());
    }

    #[test]
    fn title_line() {
        let weapons = vec![("Akbolto".to_string(), "akbolto".to_string()), ("Bolto".to_string(), "bolto".to_string())];
        let lines: Vec<String> = ["INVENTORY/MODS", "Akbolto Conci-lexicon", "+3,2 Punch Through", "-78,6% Zoom"].iter().map(|s| s.to_string()).collect();
        let (w, s, n) = parse_title(&lines, weapons.iter().map(|(n, s)| (n.as_str(), Some(s.as_str())))).unwrap();
        assert_eq!((w.as_str(), s.as_deref(), n.as_str()), ("Akbolto", Some("akbolto"), "Conci-lexicon"));
    }
}
