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

pub fn ee_log_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Warframe")
        .join("EE.log")
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
