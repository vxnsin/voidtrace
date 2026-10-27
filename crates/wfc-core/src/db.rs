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
