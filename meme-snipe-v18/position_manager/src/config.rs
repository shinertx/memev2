// position_manager/src/config.rs
use lazy_static::lazy_static;
use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Config {
    pub paper_trading_mode: bool,
    pub wallet_keypair_path: String, // Position manager needs wallet for closing trades
    pub solana_rpc_url: String,
    pub jupiter_api_url: String,
    pub signer_url: String,
    pub redis_url: String,
    pub database_path: String,
    pub trailing_stop_loss_percent: f64,
}

impl Config {
    fn load() -> Self {
        Self {
            paper_trading_mode: env::var("PAPER_TRADING_MODE")
                .unwrap_or_else(|_| "true".to_string())
                == "true",
            wallet_keypair_path: env::var("WALLET_KEYPAIR_FILENAME")
                .expect("WALLET_KEYPAIR_FILENAME must be set"),
            solana_rpc_url: env::var("SOLANA_RPC_URL").expect("SOLANA_RPC_URL must be set"),
            jupiter_api_url: env::var("JUPITER_API_URL").expect("JUPITER_API_URL must be set"),
            signer_url: env::var("SIGNER_URL").expect("SIGNER_URL must be set"),
            trailing_stop_loss_percent: env::var("TRAILING_STOP_LOSS_PERCENT")
                .expect("TRAILING_STOP_LOSS_PERCENT must be set")
                .parse()
                .unwrap(),
            database_path: env::var("DATABASE_PATH").expect("DATABASE_PATH must be set"),
            redis_url: env::var("REDIS_URL").expect("REDIS_URL must be set"),
        }
    }
}

lazy_static! {
    pub static ref CONFIG: Config = Config::load();
}
