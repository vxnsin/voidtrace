//! VoidTrace core: everything that is not UI.
//!
//! Data sources, in order of trust:
//! - DE PublicExport (items, relics, drop tables, dispositions)
//! - DE public profile endpoint (XP per item -> mastery / foundry)
//! - warframe.market v2 (prices, own listings)
//! - EE.log (game events: login, missions, reward screen, trades)
//! - Screen OCR (relic reward screen only)
//!
//! Nothing here touches the game process.

pub mod db;
pub mod export;
pub mod foundry;
pub mod inventory;
pub mod log;
pub mod market;
pub mod mastery;
pub mod memory;
pub mod ocr;
pub mod profile;
pub mod relic;
pub mod riven;
pub mod riven_ocr;
pub mod worldstate;
pub mod ws;

use std::path::PathBuf;

pub fn data_dir() -> PathBuf {
    let dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("VoidTrace");
    std::fs::create_dir_all(&dir).ok();
    dir
}

static EE_LOG_OVERRIDE: parking_lot::RwLock<Option<PathBuf>> = parking_lot::RwLock::new(None);

/// A path from the settings wins over the detected one (Linux setups differ).
pub fn set_ee_log_override(p: Option<PathBuf>) {
    *EE_LOG_OVERRIDE.write() = p;
}

pub fn ee_log_path() -> PathBuf {
    if let Some(p) = EE_LOG_OVERRIDE.read().clone() {
        return p;
    }
    #[cfg(windows)]
    {
        dirs::data_local_dir().unwrap_or_else(|| PathBuf::from(".")).join("Warframe").join("EE.log")
    }
    #[cfg(not(windows))]
    {
        // Warframe runs through Proton: the log is inside the game's prefix.
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let prefix = "steamapps/compatdata/230410/pfx/drive_c/users/steamuser/AppData/Local/Warframe/EE.log";
        let candidates = [
            home.join(".steam/steam").join(prefix),
            home.join(".local/share/Steam").join(prefix),
            home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam").join(prefix),
        ];
        candidates.iter().find(|p| p.exists()).cloned().unwrap_or_else(|| candidates[0].clone())
    }
}

/// Normalise an item name for fuzzy matching: lowercase, strip non-alnum.
pub fn norm(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == ' ')
        .collect::<String>()
        .to_ascii_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
