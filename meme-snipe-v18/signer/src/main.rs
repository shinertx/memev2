// signer/src/main.rs
use anyhow::{anyhow, Result};
use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use base64::Engine;
use shared_models::{SignRequest, SignResponse};
use solana_sdk::{
    hash::Hash,
    message::VersionedMessage,
    signature::{Keypair, Signer},
    transaction::VersionedTransaction,
};
use std::{env, fs, net::SocketAddr, sync::Arc};
use tracing::{error, info, instrument, level_filters::LevelFilter};
use tracing_subscriber::EnvFilter;

struct AppState {
    keypair: Keypair,
}

#[tokio::main]
async fn main() -> Result<()> {
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();
    tracing_subscriber::fmt().with_env_filter(filter).init();

    info!("🔒 Starting Signer Service...");

    let wallet_filename =
        env::var("WALLET_KEYPAIR_FILENAME").expect("WALLET_KEYPAIR_FILENAME must be set");
    let wallet_path = format!("/app/{}", wallet_filename);

    // Read the JSON array format wallet file
    let wallet_data = fs::read_to_string(&wallet_path)
        .map_err(|e| anyhow!("Failed to read wallet file at {}: {}", wallet_path, e))?;

    let byte_array: Vec<u8> = serde_json::from_str(&wallet_data)
        .map_err(|e| anyhow!("Failed to parse wallet JSON: {}", e))?;

    if byte_array.len() != 64 {
        return Err(anyhow!(
            "Invalid wallet file format: expected 64 bytes, got {}",
            byte_array.len()
        ));
    }

    let keypair = Keypair::from_bytes(&byte_array)
        .map_err(|e| anyhow!("Failed to create keypair from bytes: {}", e))?;

    let pubkey = keypair.pubkey();
    info!(%pubkey, "Wallet loaded successfully. This service is now ready to sign transactions.");

    let state = Arc::new(AppState { keypair });

    let app = Router::new()
        .route("/pubkey", get(get_pubkey))
        .route("/sign", post(sign_transaction))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8989));
    info!("Listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

#[instrument(skip(state), name = "get_pubkey_handler")]
async fn get_pubkey(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "pubkey": state.keypair.pubkey().to_string() }))
}

#[instrument(skip(state, request), name = "sign_transaction_handler")]
async fn sign_transaction(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SignRequest>,
) -> Result<Json<SignResponse>, StatusCode> {
    // Check if paper trading mode is enabled - reject live orders
    if std::env::var("PAPER_TRADING_MODE") == Ok("true".to_string()) {
        error!("🚫 PAPER TRADING MODE: Rejecting live transaction signing request");
        return Err(StatusCode::FORBIDDEN);
    }

    let tx_bytes = match base64::engine::general_purpose::STANDARD.decode(&request.transaction_b64)
    {
        Ok(bytes) => bytes,
        Err(e) => {
            error!(error = %e, "Failed to decode base64 transaction");
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    let mut tx: VersionedTransaction = match bincode::deserialize(&tx_bytes) {
        Ok(tx) => tx,
        Err(e) => {
            error!(error = %e, "Failed to deserialize transaction");
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    // For VersionedTransaction, we need to get the recent blockhash and sign accordingly
    let recent_blockhash = match &tx.message {
        VersionedMessage::Legacy(msg) => msg.recent_blockhash,
        VersionedMessage::V0(msg) => msg.recent_blockhash,
    };

    // Create a signature for the transaction
    let signature = state.keypair.sign_message(&tx.message.serialize());

    // Set the signature on the transaction
    if tx.signatures.is_empty() {
        tx.signatures.push(signature);
    } else {
        tx.signatures[0] = signature;
    }

    let signed_tx_bytes = match bincode::serialize(&tx) {
        Ok(bytes) => bytes,
        Err(e) => {
            error!(error = %e, "Failed to serialize signed transaction");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    info!("Transaction signed successfully.");
    Ok(Json(SignResponse {
        signed_transaction_b64: base64::engine::general_purpose::STANDARD.encode(&signed_tx_bytes),
    }))
}
