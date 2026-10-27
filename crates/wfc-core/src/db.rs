//! Local SQLite store: settings, stats datapoints, trades, relic runs, inventory.

use anyhow::Result;
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone)]
pub struct Db(Arc<Mutex<Connection>>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub id: i64,
    pub ts: i64,
    pub item: String,
    pub quantity: u32,
    pub platinum: i32,
    /// "sell" or "buy"
    pub direction: String,
    pub partner: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelicRun {
    pub id: i64,
    pub ts: i64,
    pub relic: String,
    pub rewards: Vec<String>,
    pub chosen: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Datapoint {
    pub ts: i64,
    pub platinum: i32,
    pub ducats: i32,
    pub credits: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvRow {
    pub slug: String,
    pub name: String,
    pub quantity: i32,
}

impl Db {
    pub fn open() -> Result<Self> {
        let c = Connection::open(crate::data_dir().join("voidtrace.db"))?;
        c.execute_batch(
            r#"
            PRAGMA journal_mode=WAL;
            CREATE TABLE IF NOT EXISTS settings(k TEXT PRIMARY KEY, v TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS datapoints(ts INTEGER PRIMARY KEY, platinum INTEGER, ducats INTEGER, credits INTEGER);
            CREATE TABLE IF NOT EXISTS trades(id INTEGER PRIMARY KEY, ts INTEGER, item TEXT, quantity INTEGER, platinum INTEGER, direction TEXT, partner TEXT);
            CREATE TABLE IF NOT EXISTS relic_runs(id INTEGER PRIMARY KEY, ts INTEGER, relic TEXT, rewards TEXT, chosen TEXT);
            CREATE TABLE IF NOT EXISTS inventory(slug TEXT PRIMARY KEY, name TEXT, quantity INTEGER NOT NULL DEFAULT 0);
            "#,
        )?;
        Ok(Self(Arc::new(Mutex::new(c))))
    }

    pub fn get(&self, k: &str) -> Option<String> {
        self.0
            .lock()
            .query_row("SELECT v FROM settings WHERE k=?1", [k], |r| r.get(0))
            .ok()
    }
    /// Every setting, sorted by key (for bug reports).
    pub fn settings(&self) -> Result<Vec<(String, String)>> {
        let c = self.0.lock();
        let mut s = c.prepare("SELECT k, v FROM settings ORDER BY k")?;
        let rows = s.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }
    pub fn set(&self, k: &str, v: &str) -> Result<()> {
        self.0.lock().execute(
            "INSERT INTO settings(k,v) VALUES(?1,?2) ON CONFLICT(k) DO UPDATE SET v=excluded.v",
            [k, v],
        )?;
        Ok(())
    }

    pub fn add_datapoint(&self, d: &Datapoint) -> Result<()> {
        self.0.lock().execute(
            "INSERT OR REPLACE INTO datapoints VALUES(?1,?2,?3,?4)",
            params![d.ts, d.platinum, d.ducats, d.credits],
        )?;
        Ok(())
    }
    pub fn datapoints(&self, limit: usize) -> Result<Vec<Datapoint>> {
        let c = self.0.lock();
        let mut s = c.prepare("SELECT ts,platinum,ducats,credits FROM datapoints ORDER BY ts DESC LIMIT ?1")?;
        let rows = s.query_map([limit as i64], |r| {
            Ok(Datapoint { ts: r.get(0)?, platinum: r.get(1)?, ducats: r.get(2)?, credits: r.get(3)? })
        })?;
        Ok(rows.filter_map(Result::ok).collect())
    }

    pub fn add_trade(&self, t: &Trade) -> Result<i64> {
        let c = self.0.lock();
        c.execute(
            "INSERT INTO trades(ts,item,quantity,platinum,direction,partner) VALUES(?1,?2,?3,?4,?5,?6)",
            params![t.ts, t.item, t.quantity, t.platinum, t.direction, t.partner],
        )?;
        Ok(c.last_insert_rowid())
    }
    pub fn trades(&self) -> Result<Vec<Trade>> {
        let c = self.0.lock();
        let mut s = c.prepare("SELECT id,ts,item,quantity,platinum,direction,partner FROM trades ORDER BY ts DESC")?;
        let rows = s.query_map([], |r| {
            Ok(Trade {
                id: r.get(0)?, ts: r.get(1)?, item: r.get(2)?, quantity: r.get(3)?,
                platinum: r.get(4)?, direction: r.get(5)?, partner: r.get(6)?,
            })
        })?;
        Ok(rows.filter_map(Result::ok).collect())
    }
    pub fn delete_trade(&self, id: i64) -> Result<()> {
        self.0.lock().execute("DELETE FROM trades WHERE id=?1", [id])?;
        Ok(())
    }

    pub fn add_relic_run(&self, relic: &str, rewards: &[String], chosen: Option<&str>) -> Result<i64> {
        let c = self.0.lock();
        c.execute(
            "INSERT INTO relic_runs(ts,relic,rewards,chosen) VALUES(?1,?2,?3,?4)",
            params![chrono::Utc::now().timestamp(), relic, serde_json::to_string(rewards)?, chosen],
        )?;
        Ok(c.last_insert_rowid())
    }
    pub fn relic_runs(&self, limit: usize) -> Result<Vec<RelicRun>> {
        let c = self.0.lock();
        let mut s = c.prepare("SELECT id,ts,relic,rewards,chosen FROM relic_runs ORDER BY ts DESC LIMIT ?1")?;
        let rows = s.query_map([limit as i64], |r| {
            let rewards: String = r.get(3)?;
            Ok(RelicRun {
                id: r.get(0)?, ts: r.get(1)?, relic: r.get(2)?,
                rewards: serde_json::from_str(&rewards).unwrap_or_default(),
                chosen: r.get(4)?,
            })
        })?;
        Ok(rows.filter_map(Result::ok).collect())
    }

    pub fn inventory(&self) -> Result<Vec<InvRow>> {
        let c = self.0.lock();
        let mut s = c.prepare("SELECT slug,name,quantity FROM inventory WHERE quantity<>0 ORDER BY name")?;
        let rows = s.query_map([], |r| Ok(InvRow { slug: r.get(0)?, name: r.get(1)?, quantity: r.get(2)? }))?;
        Ok(rows.filter_map(Result::ok).collect())
    }
    pub fn inventory_add(&self, slug: &str, name: &str, delta: i32) -> Result<i32> {
        let c = self.0.lock();
        c.execute(
            "INSERT INTO inventory(slug,name,quantity) VALUES(?1,?2,?3)
             ON CONFLICT(slug) DO UPDATE SET quantity=quantity+excluded.quantity, name=excluded.name",
            params![slug, name, delta],
        )?;
        Ok(c.query_row("SELECT quantity FROM inventory WHERE slug=?1", [slug], |r| r.get(0))?)
    }
    /// Replace the whole ledger with a game snapshot.
    pub fn inventory_replace(&self, rows: &[(String, String, i32)]) -> Result<()> {
        let mut c = self.0.lock();
        let tx = c.transaction()?;
        tx.execute("DELETE FROM inventory", [])?;
        {
            let mut s = tx.prepare("INSERT INTO inventory(slug,name,quantity) VALUES(?1,?2,?3)")?;
            for (slug, name, qty) in rows {
                s.execute(params![slug, name, qty])?;
            }
        }
        tx.commit()?;
        Ok(())
    }
    pub fn inventory_set(&self, slug: &str, name: &str, qty: i32) -> Result<()> {
        self.0.lock().execute(
            "INSERT INTO inventory(slug,name,quantity) VALUES(?1,?2,?3)
             ON CONFLICT(slug) DO UPDATE SET quantity=excluded.quantity, name=excluded.name",
            params![slug, name, qty],
        )?;
        Ok(())
    }
}

/// Everything VoidTrace records, as one JSON document (export / import).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Backup {
    pub voidtrace_export: u32,
    pub version: String,
    pub exported: i64,
    pub trades: Vec<Trade>,
    /// Plat / ducats / credits from the inventory syncs.
    pub history: Vec<Datapoint>,
    pub relic_runs: Vec<RelicRun>,
    pub inventory: Vec<InvRow>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct Imported {
    pub trades: usize,
    pub history: usize,
    pub relic_runs: usize,
    pub inventory: usize,
}

impl Db {
    pub fn backup(&self, version: &str) -> Result<Backup> {
        Ok(Backup {
            voidtrace_export: 1,
            version: version.to_string(),
            exported: chrono::Utc::now().timestamp(),
            trades: self.trades()?,
            history: self.datapoints(usize::MAX >> 1)?,
            relic_runs: self.relic_runs(usize::MAX >> 1)?,
            inventory: self.inventory()?,
        })
    }

    /// Merge a backup into this database: rows that are already here (same
    /// time and content) are skipped, so importing twice changes nothing. The
    /// inventory is only taken over when there is none yet.
    pub fn restore(&self, b: &Backup) -> Result<Imported> {
        let mut c = self.0.lock();
        let tx = c.transaction()?;
        let mut n = Imported::default();
        for t in &b.trades {
            let dup: i64 = tx.query_row(
                "SELECT COUNT(*) FROM trades WHERE ts=?1 AND item=?2 AND platinum=?3 AND direction=?4",
                params![t.ts, t.item, t.platinum, t.direction],
                |r| r.get(0),
            )?;
            if dup == 0 {
                tx.execute(
                    "INSERT INTO trades(ts,item,quantity,platinum,direction,partner) VALUES(?1,?2,?3,?4,?5,?6)",
                    params![t.ts, t.item, t.quantity, t.platinum, t.direction, t.partner],
                )?;
                n.trades += 1;
            }
        }
        for d in &b.history {
            n.history += tx.execute("INSERT OR IGNORE INTO datapoints VALUES(?1,?2,?3,?4)", params![d.ts, d.platinum, d.ducats, d.credits])?;
        }
        for r in &b.relic_runs {
            let dup: i64 = tx.query_row("SELECT COUNT(*) FROM relic_runs WHERE ts=?1 AND relic=?2", params![r.ts, r.relic], |x| x.get(0))?;
            if dup == 0 {
                tx.execute(
                    "INSERT INTO relic_runs(ts,relic,rewards,chosen) VALUES(?1,?2,?3,?4)",
                    params![r.ts, r.relic, serde_json::to_string(&r.rewards)?, r.chosen],
                )?;
                n.relic_runs += 1;
            }
        }
        let have: i64 = tx.query_row("SELECT COUNT(*) FROM inventory WHERE quantity<>0", [], |r| r.get(0))?;
        if have == 0 {
            for i in &b.inventory {
                tx.execute("INSERT OR REPLACE INTO inventory(slug,name,quantity) VALUES(?1,?2,?3)", params![i.slug, i.name, i.quantity])?;
                n.inventory += 1;
            }
        }
        tx.commit()?;
        Ok(n)
    }
}

fn csv_field(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}
fn csv(header: &[&str], rows: impl Iterator<Item = Vec<String>>) -> String {
    let mut out = header.join(",") + "\r\n";
    for r in rows {
        out += &r.iter().map(|f| csv_field(f)).collect::<Vec<_>>().join(",");
        out += "\r\n";
    }
    out
}
fn iso(ts: i64) -> String {
    chrono::DateTime::from_timestamp(ts, 0).map(|t| t.to_rfc3339()).unwrap_or_default()
}

impl Backup {
    /// One CSV per table next to `base` (`<name>-trades.csv`, …); returns the files.
    pub fn write_csv(&self, base: &std::path::Path) -> Result<Vec<std::path::PathBuf>> {
        let stem = base.file_stem().and_then(|s| s.to_str()).unwrap_or("voidtrace").to_string();
        let dir = base.parent().map(|p| p.to_path_buf()).unwrap_or_default();
        let files = [
            ("trades", csv(&["time", "direction", "item", "quantity", "platinum", "partner"], self.trades.iter().map(|t| {
                vec![iso(t.ts), t.direction.clone(), t.item.clone(), t.quantity.to_string(), t.platinum.to_string(), t.partner.clone()]
            }))),
            ("history", csv(&["time", "platinum", "ducats", "credits"], self.history.iter().map(|d| {
                vec![iso(d.ts), d.platinum.to_string(), d.ducats.to_string(), d.credits.to_string()]
            }))),
            ("relic-runs", csv(&["time", "relic", "rewards", "chosen"], self.relic_runs.iter().map(|r| {
                vec![iso(r.ts), r.relic.clone(), r.rewards.join(" | "), r.chosen.clone().unwrap_or_default()]
            }))),
            ("inventory", csv(&["slug", "name", "quantity"], self.inventory.iter().map(|i| vec![i.slug.clone(), i.name.clone(), i.quantity.to_string()]))),
        ];
        let mut out = vec![];
        for (name, body) in files {
            let p = dir.join(format!("{stem}-{name}.csv"));
            // BOM so Excel opens it as UTF-8.
            std::fs::write(&p, format!("\u{feff}{body}"))?;
            out.push(p);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_quotes_what_needs_quoting() {
        assert_eq!(csv_field("Ash Prime Set"), "Ash Prime Set");
        assert_eq!(csv_field("a,b"), "\"a,b\"");
        assert_eq!(csv_field("say \"hi\""), "\"say \"\"hi\"\"\"");
    }
}
