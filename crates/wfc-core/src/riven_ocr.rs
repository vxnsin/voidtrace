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
    // Faction damage is always shown as a multiplier; OCR sometimes drops the "x".
    let faction = matches!(key, "grineer" | "corpus" | "infested");
    let multiplier = faction || prefix.eq_ignore_ascii_case("x") || prefix.ends_with('x') || prefix.ends_with('X');
    // "xl 152" is x1.52 without its decimal point; drop it so the other
    // reading supplies the stat in merge.
    if faction && !(0.1..=3.0).contains(&num) {
        return None;
    }
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

/// Card text plus the "Fits in" box. The card title wraps over two lines on the
/// cycling screen ("Wolf Sledge Tempi-" / "loctiata"), so neighbouring lines are
/// joined before the title is looked up; the "Fits in" weapon is the fallback.
pub fn parse_card(ex: &Export, weapons: &[(String, String)], card: &[String], fits_in: &[String]) -> Option<RivenCard> {
    let stats: Vec<StatInput> = card.iter().filter_map(|l| parse_stat(l)).take(4).collect();
    if stats.len() < 2 {
        return None;
    }
    // Lines above the first stat are the title.
    let first_stat = card.iter().position(|l| parse_stat(l).is_some()).unwrap_or(card.len());
    let title: String = card[..first_stat]
        .iter()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .fold(String::new(), |acc, l| if acc.ends_with('-') || acc.is_empty() { acc + l } else { acc + " " + l });
    let pairs = |list: &[String]| -> Vec<String> {
        // The joined title first: on equal weapon matches the first line wins.
        let mut v = vec![title.clone()];
        v.extend_from_slice(list);
        v
    };
    let known = |name: &str| -> Option<(String, Option<String>)> {
        let n = crate::norm(name);
        if weapons.is_empty() {
            ex.weapons.iter().find(|w| crate::norm(&w.name) == n).map(|w| (w.name.clone(), None))
        } else {
            weapons.iter().find(|(w, _)| crate::norm(w) == n).map(|(w, s)| (w.clone(), Some(s.clone())))
        }
    };
    let lines = pairs(card);
    let from_title = if weapons.is_empty() {
        parse_title(&lines, ex.weapons.iter().map(|w| (w.name.as_str(), None)))
    } else {
        parse_title(&lines, weapons.iter().map(|(n, s)| (n.as_str(), Some(s.as_str()))))
    };
    if let Some((weapon, weapon_slug, name)) = from_title {
        return Some(RivenCard { weapon, weapon_slug, name, stats });
    }
    // "FITS IN" then the weapon name on the next line(s).
    let idx = fits_in.iter().position(|l| crate::norm(l).contains("fits in"));
    let candidates: Vec<&String> = match idx {
        Some(i) => fits_in[i + 1..].iter().collect(),
        None => fits_in.iter().collect(),
    };
    // The name can wrap: "Wolf" / "Sledge".
    let (weapon, weapon_slug) = candidates
        .iter()
        .find_map(|l| known(l))
        .or_else(|| candidates.windows(2).find_map(|w| known(&format!("{} {}", w[0].trim(), w[1].trim()))))?;
    let wl = weapon.to_ascii_lowercase();
    let name = title
        .to_ascii_lowercase()
        .strip_prefix(&wl)
        .map(|_| title[weapon.len()..].trim().to_string())
        .unwrap_or_else(|| title.clone());
    Some(RivenCard { weapon, weapon_slug, name, stats })
}

#[cfg(test)]
mod card_tests {
    use super::*;

    #[test]
    fn wrapped_title_and_fits_in() {
        let ex = Export::default();
        let weapons = vec![("Wolf Sledge".to_string(), "wolf_sledge".to_string())];
        let card: Vec<String> = ["Wolf Sledge Tempi-", "loctiata", "+172,3% Melee Damage", "+9,2s Combo Duration", "+2,1 Range", "-50,2% Attack Speed", "MR 14"]
            .iter().map(|s| s.to_string()).collect();
        let c = parse_card(&ex, &weapons, &card, &[]).unwrap();
        assert_eq!((c.weapon.as_str(), c.name.as_str(), c.stats.len()), ("Wolf Sledge", "Tempi-loctiata", 4));
        assert!(c.stats[3].negative && c.stats[3].stat == "fire_rate");
        // title unreadable -> weapon from "Fits in"
        let card2: Vec<String> = ["W0lf Sl3dge Tempi-", "loctiata", "+172,3% Melee Damage", "+2,1 Range"].iter().map(|s| s.to_string()).collect();
        let fits: Vec<String> = ["FITS IN", "Wolf Sledge"].iter().map(|s| s.to_string()).collect();
        let c = parse_card(&ex, &weapons, &card2, &fits).unwrap();
        assert_eq!(c.weapon, "Wolf Sledge");
    }
}

/// Two OCR readings of the same card: keep the one with more stats and fill
/// stats it missed from the other (same order as on the card).
pub fn merge(a: Option<RivenCard>, b: Option<RivenCard>) -> Option<RivenCard> {
    match (a, b) {
        (Some(a), Some(b)) => {
            let (mut base, other) = if b.stats.len() > a.stats.len() { (b, a) } else { (a, b) };
            for s in other.stats {
                if base.stats.len() < 4 && !base.stats.iter().any(|x| x.stat == s.stat) {
                    base.stats.push(s);
                }
            }
            Some(base)
        }
        (a, b) => a.or(b),
    }
}

#[cfg(test)]
mod ocr_real_tests {
    use super::*;

    // Lines read from a real 1080p cycling screen (Color x1).
    #[test]
    fn real_confirm_screen() {
        let ex = Export::default();
        let weapons = vec![("Wolf Sledge".to_string(), "wolf_sledge".to_string())];
        let l = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let a = parse_card(&ex, &weapons, &l(&["Wolf Sledge Argi-", "tempitron", "+8,8s Combo Duration", "x 1,52 Damage to Grineer", "xl Damage to Corpus", "0,63 Damage to Infested", "MR 14"]), &[]);
        let b = parse_card(&ex, &weapons, &l(&["Wolf Sledge Argi-", "tempitron", "+8,8s Combo Duration", "xl 152 Damage to Grineer", "x1,48 Damage to Corpus", "x0,63 Damage to Infested"]), &[]);
        let c = merge(a, b).unwrap();
        assert_eq!(c.name, "Argi-tempitron");
        let keys: Vec<&str> = c.stats.iter().map(|s| s.stat.as_str()).collect();
        assert!(keys.contains(&"combo_duration") && keys.contains(&"grineer") && keys.contains(&"corpus") && keys.contains(&"infested"), "{keys:?}");
        let inf = c.stats.iter().find(|s| s.stat == "infested").unwrap();
        assert!(inf.negative && (inf.value + 37.0).abs() < 0.1, "{inf:?}");
    }
}
