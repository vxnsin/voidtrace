//! EE.log tailer. Emits high-level game events.
//!
//! Patterns live in `patterns.toml`-style defaults below and can be overridden
//! from the settings UI, because DE changes log strings between updates.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GameEvent {
    LoggedIn { name: String, id: Option<String> },
    MissionStart { name: String },
    MissionEnd,
    /// Relic reward screen is up; OCR should run now.
    RewardScreen,
    /// Reward timer finished; a pick has been made or auto-selected.
    RewardDone,
    TradeComplete,
    RelicOpened { relic: String },
    /// The relic selection screen before a fissure (pattern `relic_select`;
    /// empty until the line is known from a real EE.log).
    RelicSelect,
    /// The Warframe process appeared (process watcher).
    GameStarted,
    /// The game window gained or lost the foreground (process watcher).
    GameFocus { focused: bool },
    /// The Warframe process went away. Comes from the process watcher, not
    /// the log: a crash or a killed process writes no shutdown line.
    GameClosed,
    Raw { line: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Patterns {
    pub logged_in: String,
    pub mission_start: String,
    pub mission_end: String,
    pub reward_screen: String,
    pub reward_done: String,
    pub trade_complete: String,
    pub relic_opened: String,
    /// Empty = off.
    #[serde(default)]
    pub relic_select: String,
}

impl Default for Patterns {
    fn default() -> Self {
        Self {
            // Older builds append the account id: "Logged in Name (5fb6...)".
            logged_in: r"Logged in (\S+)(?: \(([0-9a-f]{24})\))?".into(),
            mission_start: r#"Sys \[Info\]: Loading level: .*?/([A-Za-z0-9_]+)"#.into(),
            mission_end: r"Sys \[Info\]: EOM\b".into(),
            // Verified against a live EE.log: the four cards exist once
            // "Got rewards" is logged; the .swf also appears in loader lines, so
            // it is not a usable trigger.
            reward_screen: r"ProjectionRewardChoice\.lua: Got rewards".into(),
            reward_done: r"ProjectionRewardChoice\.lua: Relic reward screen shut down".into(),
            trade_complete: r"Trade complete|TradingDialog.*?completed".into(),
            // Last relic loaded before the fissure; refinement suffix stripped.
            relic_opened: r"Projections/(T\dVoidProjection\w+?)(?:Bronze|Silver|Gold|Platinum)\)".into(),
            relic_select: String::new(),
        }
    }
}

struct Compiled {
    logged_in: Regex,
    mission_start: Regex,
    mission_end: Regex,
    reward_screen: Regex,
    reward_done: Regex,
    trade_complete: Regex,
    relic_opened: Regex,
    relic_select: Option<Regex>,
}

impl Compiled {
    fn new(p: &Patterns) -> anyhow::Result<Self> {
        Ok(Self {
            logged_in: Regex::new(&p.logged_in)?,
            mission_start: Regex::new(&p.mission_start)?,
            mission_end: Regex::new(&p.mission_end)?,
            reward_screen: Regex::new(&p.reward_screen)?,
            reward_done: Regex::new(&p.reward_done)?,
            trade_complete: Regex::new(&p.trade_complete)?,
            relic_opened: Regex::new(&p.relic_opened)?,
            relic_select: if p.relic_select.trim().is_empty() { None } else { Some(Regex::new(&p.relic_select)?) },
        })
    }

    fn classify(&self, line: &str) -> Option<GameEvent> {
        if let Some(c) = self.logged_in.captures(line) {
            return Some(GameEvent::LoggedIn {
                name: c[1].to_string(),
                id: c.get(2).map(|m| m.as_str().to_string()),
            });
        }
        // Verified sample lines (EE.log, 2026-10):
        //   Sys [Info]: Logged in Vensin
        //   Script [Info]: ProjectionRewardChoice.lua: Got rewards
        //   Script [Info]: ProjectionRewardChoice.lua: Relic reward screen shut down
        //   Sys [Info]: EOM missionLocationUnlocked=1
        //   ResourceLoader ... (/Lotus/Types/Game/Projections/T2VoidProjectionProteaPrimeABronze) ...
        if self.reward_screen.is_match(line) {
            return Some(GameEvent::RewardScreen);
        }
        if self.reward_done.is_match(line) {
            return Some(GameEvent::RewardDone);
        }
        if self.relic_select.as_ref().is_some_and(|r| r.is_match(line)) {
            return Some(GameEvent::RelicSelect);
        }
        if let Some(c) = self.relic_opened.captures(line) {
            return Some(GameEvent::RelicOpened { relic: c[1].to_string() });
        }
        if self.trade_complete.is_match(line) {
            return Some(GameEvent::TradeComplete);
        }
        if self.mission_end.is_match(line) {
            return Some(GameEvent::MissionEnd);
        }
        if let Some(c) = self.mission_start.captures(line) {
            return Some(GameEvent::MissionStart { name: c[1].to_string() });
        }
        None
    }
}

/// Polls the process list (every 5 s) and the foreground window (every second)
/// and sends `GameStarted` / `GameClosed` / `GameFocus` on the edges. The process
/// state at app start is the baseline: an already running game sends no start.
pub fn spawn_process_watcher(tx: broadcast::Sender<GameEvent>) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut pid = crate::memory::find_pid();
        let mut focused = false;
        for tick in 1u64.. {
            std::thread::sleep(Duration::from_secs(1));
            if tick % 5 == 0 {
                let now = crate::memory::find_pid();
                if pid.is_some() && now.is_none() {
                    let _ = tx.send(GameEvent::GameClosed);
                } else if pid.is_none() && now.is_some() {
                    let _ = tx.send(GameEvent::GameStarted);
                }
                pid = now;
            }
            let f = pid.is_some() && crate::memory::foreground_pid() == pid;
            if f != focused {
                focused = f;
                let _ = tx.send(GameEvent::GameFocus { focused });
            }
        }
    })
}

/// Tails `path` from its current end and broadcasts events. Survives the game
/// truncating the file on restart (offset resets when the file shrinks).
pub fn spawn_tailer(
    path: PathBuf,
    patterns: Patterns,
    tx: broadcast::Sender<GameEvent>,
    raw: bool,
) -> anyhow::Result<std::thread::JoinHandle<()>> {
    let compiled = Compiled::new(&patterns)?;
    Ok(std::thread::spawn(move || {
        let mut offset: u64 = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        let mut partial = String::new();
        loop {
            std::thread::sleep(Duration::from_millis(400));
            let Ok(mut f) = std::fs::File::open(&path) else { continue };
            let len = f.metadata().map(|m| m.len()).unwrap_or(0);
            if len < offset {
                offset = 0;
                partial.clear();
            }
            if len == offset {
                continue;
            }
            if f.seek(SeekFrom::Start(offset)).is_err() {
                continue;
            }
            let mut buf = Vec::new();
            if f.read_to_end(&mut buf).is_err() {
                continue;
            }
            offset = len;
            partial.push_str(&String::from_utf8_lossy(&buf));
            let mut lines: Vec<&str> = partial.split('\n').collect();
            let rest = lines.pop().unwrap_or("").to_string();
            for line in lines {
                let line = line.trim_end_matches('\r');
                if let Some(ev) = compiled.classify(line) {
                    let _ = tx.send(ev);
                } else if raw {
                    let _ = tx.send(GameEvent::Raw { line: line.to_string() });
                }
            }
            partial = rest;
        }
    }))
}
