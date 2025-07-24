// executor/src/database.rs
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use shared_models::OrderDetails;
use std::path::Path;
use tracing::info;

// --- Trade Record Struct ---
#[derive(Debug, Clone)] // Added Clone for position_manager
pub struct TradeRecord {
    pub id: i64,
    pub strategy_id: String,
    pub token_address: String,
    pub symbol: String, // Stored for dashboard convenience
    pub amount_usd: f64,
    pub status: String,
    pub signature: Option<String>,
    pub entry_time: i64,
    pub entry_price_usd: f64,
    pub close_time: Option<i64>,
    pub close_price_usd: Option<f64>,
    pub pnl_usd: Option<f64>,
    pub confidence: f64,
    pub side: String,                   // NEW: Store trade side (Long/Short)
    pub highest_price_usd: Option<f64>, // NEW: For trailing stop-loss
    pub mode: String,                   // NEW: Paper vs Live mode
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
        let conn = Connection::open(path)
            .with_context(|| format!("Failed to open database at {}", db_path))?;
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
                side TEXT NOT NULL, -- NEW
                highest_price_usd REAL, -- NEW
                mode TEXT NOT NULL DEFAULT 'Paper' -- NEW: Track Paper vs Live trades
            )",
            [],
        )?;

        // Add mode column if it doesn't exist (migration for existing databases)
        let mut stmt = conn.prepare("PRAGMA table_info(trades)")?;
        let has_mode_column = stmt
            .query_map([], |row| row.get::<_, String>(1))?
            .any(|col_name| col_name.as_deref() == Ok("mode"));

        if !has_mode_column {
            conn.execute(
                "ALTER TABLE trades ADD COLUMN mode TEXT NOT NULL DEFAULT 'Paper'",
                [],
            )?;
        }

        Ok(())
    }

    pub fn log_trade_attempt(
        &self,
        details: &OrderDetails,
        strategy_id: &str,
        entry_price_usd: f64,
        mode: &str,
    ) -> Result<i64> {
        let now: DateTime<Utc> = Utc::now();
        self.conn.execute(
            "INSERT INTO trades (strategy_id, token_address, symbol, amount_usd, status, entry_time, entry_price_usd, confidence, side, highest_price_usd, mode)
             VALUES (?1, ?2, ?3, ?4, 'PENDING', ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                strategy_id,
                details.token_address,
                details.token_address, // Use address as symbol for now, can be updated later
                details.suggested_size_usd,
                now.timestamp(),
                entry_price_usd,
                details.confidence,
                details.side.to_string(),
                entry_price_usd, // Initialize highest_price with entry price
                mode,
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn open_trade(&self, trade_id: i64, signature: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE trades SET status = 'OPEN', signature = ?1 WHERE id = ?2",
            params![signature, trade_id],
        )?;
        Ok(())
    }

    pub fn get_all_trades(&self) -> Result<Vec<TradeRecord>> {
        let mut stmt = self.conn.prepare("SELECT id, strategy_id, token_address, symbol, amount_usd, status, signature, entry_time, entry_price_usd, close_time, close_price_usd, pnl_usd, confidence, side, highest_price_usd, mode FROM trades ORDER BY entry_time DESC")?;
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
                mode: row.get(15)?,
            })
        })?;

        trades_iter
            .collect::<Result<Vec<TradeRecord>, rusqlite::Error>>()
            .map_err(anyhow::Error::from)
    }

    pub fn get_open_trades(&self) -> Result<Vec<TradeRecord>> {
        // NEW: For position_manager
        let mut stmt = self.conn.prepare("SELECT id, strategy_id, token_address, symbol, amount_usd, status, signature, entry_time, entry_price_usd, close_time, close_price_usd, pnl_usd, confidence, side, highest_price_usd, mode FROM trades WHERE status = 'OPEN'")?;
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
                mode: row.get(15)?,
            })
        })?;
        trades_iter
            .collect::<Result<Vec<TradeRecord>, rusqlite::Error>>()
            .map_err(anyhow::Error::from)
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
        // NEW: For position_manager
        self.conn.execute(
            "UPDATE trades SET highest_price_usd = ?1 WHERE id = ?2",
            params![new_highest_price, trade_id],
        )?;
        Ok(())
    }

    pub fn get_total_pnl(&self) -> Result<f64> {
        let total: Option<f64> = self.conn.query_row(
            "SELECT SUM(pnl_usd) FROM trades WHERE status LIKE 'CLOSED_%'",
            [],
            |row| row.get(0),
        )?;
        Ok(total.unwrap_or(0.0))
    }
}
