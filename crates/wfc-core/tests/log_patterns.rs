//! Default log patterns against real EE.log lines.

use regex::Regex;
use wfc_core::log::Patterns;

fn cap(re: &str, line: &str) -> Option<String> {
    Regex::new(re).unwrap().captures(line).map(|c| c.get(1).map(|m| m.as_str().to_string()).unwrap_or_default())
}

#[test]
fn defaults_match_real_lines() {
    let p = Patterns::default();
    assert_eq!(cap(&p.logged_in, "36.890 Sys [Info]: Logged in Vensin").as_deref(), Some("Vensin"));
    let re = Regex::new(&p.logged_in).unwrap();
    let c = re.captures("36.890 Sys [Info]: Logged in Vensin (5fb6bd1e9ee0254be87344d6)").unwrap();
    assert_eq!(&c[1], "Vensin");
    assert_eq!(c.get(2).map(|m| m.as_str()), Some("5fb6bd1e9ee0254be87344d6"));
    assert!(cap(&p.reward_screen, "1133.219 Script [Info]: ProjectionRewardChoice.lua: Got rewards").is_some());
    assert!(cap(&p.reward_screen, "842.185 Sys [Info]: ResourceLoader 0x1 (/Lotus/Interface/ProjectionRewardChoice.swf) Found 5,784 items").is_none());
    assert!(cap(&p.reward_done, "1148.222 Script [Info]: ProjectionRewardChoice.lua: Relic reward screen shut down").is_some());
    assert!(cap(&p.mission_end, "740.085 Sys [Info]: EOM missionLocationUnlocked=1").is_some());
    assert_eq!(
        cap(&p.relic_opened, "840.083 Sys [Info]: Resloader 0x1 (/Lotus/Types/Game/Projections/T2VoidProjectionProteaPrimeABronze) starting").as_deref(),
        Some("T2VoidProjectionProteaPrimeA")
    );
}
