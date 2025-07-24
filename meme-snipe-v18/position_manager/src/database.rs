// position_manager/src/database.rs
// This is a copy of executor/src/database.rs for the position_manager
// to ensure it has its own independent DB connection and logic.
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use std::path::Path;
use tracing::info;

// --- Trade Record Struct ---
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct TradeRecord {
    pub id: i64,
    pub strategy_id: String,
    pub token_address: String,
    pub symbol: String,
    pub amount_usd: f64,
    pub status: String,
    pub signature: Option<String>,
    pub entry_time: i64,
    pub entry_price_usd: f64,
    pub close_time: Option<i64>,
    pub close_price_usd: Option<f64>,
    pub pnl_usd: Option<f64>,
    pub confidence: f64,
    pub side: String,
    pub highest_price_usd: Option<f64>,
}

// --- Database Manager ---
pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new(db_path: &str) -> Result<Self> {
        let path = Path::new(db_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(db_path).with_context(|| format!("Failed to open database at {db_path}"))?;
        info!("Database opened at {}", db_path);
        Self::init_db(&conn)?;
        Ok(Self { conn })
    }

    fn init_db(conn: &Connection) -> Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS trades (
                id INTEGER PRIMARY KEY,
                strategy_id TEXT NOT NULL,
                token_address TEXT NOT NULL,
                symbol TEXT NOT NULL,
                amount_usd REAL NOT NULL,
                status TEXT NOT NULL, -- PENDING, OPEN, CLOSED_PROFIT, CLOSED_LOSS, CANCELED
                signature TEXT,
                entry_time INTEGER NOT NULL,
                entry_price_usd REAL NOT NULL,
                close_time INTEGER,
                close_price_usd REAL,
                pnl_usd REAL,
                confidence REAL NOT NULL,
                side TEXT NOT NULL,
                highest_price_usd REAL
            )",
            [],
        )?;
        Ok(())
    }

    pub fn get_open_trades(&self) -> Result<Vec<TradeRecord>> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM trades WHERE status = 'OPEN'")?;
        let trades_iter = stmt.query_map([], |row| {
            Ok(TradeRecord {
                id: row.get(0)?,
                strategy_id: row.get(1)?,
                token_address: row.get(2)?,
                symbol: row.get(3)?,
                amount_usd: row.get(4)?,
                status: row.get(5)?,
                signature: row.get(6)?,
                entry_time: row.get(7)?,
                entry_price_usd: row.get(8)?,
                close_time: row.get(9)?,
                close_price_usd: row.get(10)?,
                pnl_usd: row.get(11)?,
                confidence: row.get(12)?,
                side: row.get(13)?,
                highest_price_usd: row.get(14)?,
            })
        })?;
        trades_iter
            .collect::<Result<Vec<TradeRecord>, rusqlite::Error>>()
            .map_err(anyhow::Error::from)
    }

    #[allow(dead_code)]
    pub fn update_trade_status(&self, trade_id: i64, status: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE trades SET status = ?1 WHERE id = ?2",
            params![status, trade_id],
        )?;
        Ok(())
    }

    pub fn update_trade_pnl(
        &self,
        trade_id: i64,
        status: &str,
        close_price_usd: f64,
        pnl_usd: f64,
    ) -> Result<()> {
        let now: DateTime<Utc> = Utc::now();
        self.conn.execute(
            "UPDATE trades SET status = ?1, close_time = ?2, close_price_usd = ?3, pnl_usd = ?4 WHERE id = ?5",
            params![status, now.timestamp(), close_price_usd, pnl_usd, trade_id],
        )?;
        Ok(())
    }

    pub fn update_highest_price(&self, trade_id: i64, new_highest_price: f64) -> Result<()> {
        self.conn.execute(
            "UPDATE trades SET highest_price_usd = ?1 WHERE id = ?2",
            params![new_highest_price, trade_id],
        )?;
        Ok(())
    }
}
