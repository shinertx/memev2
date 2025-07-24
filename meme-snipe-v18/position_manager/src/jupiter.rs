// position_manager/src/jupiter.rs
// This is a copy of executor/src/jupiter.rs for the position_manager
// to ensure it has its own independent API client.
use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use solana_sdk::{pubkey::Pubkey, transaction::VersionedTransaction};
use std::time::Duration;
use tracing::info;

#[derive(Debug, Deserialize, Serialize, Clone)]
#[allow(dead_code)]
#[serde(rename_all = "camelCase")]
pub struct JupiterQuote {
    pub out_amount: String,
    #[serde(rename = "marketInfos")]
    pub market_infos: Vec<MarketInfo>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[allow(dead_code)]
#[serde(rename_all = "camelCase")]
pub struct MarketInfo {
    pub lp_fee: LpFee,
    pub liquidity: f64,
    pub label: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[allow(dead_code)]
#[serde(rename_all = "camelCase")]
pub struct LpFee {
    pub amount: String,
    pub mint: String,
    pub pct: f64,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
#[serde(rename_all = "camelCase")]
pub struct JupiterQuoteResponse {
    pub data: Vec<JupiterQuote>,
}

#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct QuoteRequest {
    pub quote_response: JupiterQuote,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct QuoteResult {
    pub out_amount: String,
    pub other_amount_threshold: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct JupiterSwapResponse {
    pub swap_transaction: String,
}

#[derive(Clone)]
pub struct JupiterClient {
    client: Client,
    api_url: String,
}

impl JupiterClient {
    pub fn new(api_url: String) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(15))
                .build()
                .unwrap(),
            api_url,
        }
    }

    #[allow(dead_code)]
    pub async fn get_quote(
        &self,
        input_mint: &str,
        output_mint: &str,
        amount: u64,
        slippage_bps: u16,
    ) -> Result<JupiterQuote> {
        let url = format!(
            "{}/quote?inputMint={}&outputMint={}&amount={}&slippageBps={}",
            self.api_url, input_mint, output_mint, amount, slippage_bps
        );

        let response: JupiterQuoteResponse = self.client.get(&url).send().await?.json().await?;
        let best_route = response
            .data
            .first()
            .ok_or_else(|| anyhow!("No route found by Jupiter for {}", output_mint))?;

        Ok(best_route.clone())
    }

    pub async fn get_swap_transaction(
        &self,
        user_pubkey: &Pubkey,
        output_mint: &str,
        amount_usd_to_swap: f64,
        slippage_bps: u16,
    ) -> Result<String> {
        let amount_sol_approx = amount_usd_to_swap / 150.0; // Placeholder SOL price for Jupiter's internal calculation.
        let amount_lamports = (amount_sol_approx * 1_000_000_000.0) as u64;

        let quote_url = format!(
            "{}/quote?inputMint=So11111111111111111111111111111111111111112&outputMint={}&amount={}&slippageBps={}",
            self.api_url, output_mint, amount_lamports, slippage_bps
        );
        let quote_response: serde_json::Value =
            self.client.get(&quote_url).send().await?.json().await?;

        let swap_payload = serde_json::json!({
            "quoteResponse": quote_response,
            "userPublicKey": user_pubkey.to_string(),
            "wrapAndUnwrapSol": true,
        });

        let swap_url = format!("{}/swap", self.api_url);
        let response: JupiterSwapResponse = self
            .client
            .post(swap_url)
            .json(&swap_payload)
            .send()
            .await?
            .json()
            .await?;
        info!(
            "Generated Jupiter swap transaction for {} USD.",
            amount_usd_to_swap
        );
        Ok(response.swap_transaction)
    }
}

use base64::{engine::general_purpose, Engine as _};

pub fn deserialize_transaction(tx_b64: &str) -> Result<VersionedTransaction> {
    let tx_bytes = general_purpose::STANDARD.decode(tx_b64)?;
    bincode::deserialize(&tx_bytes).context("Failed to deserialize transaction")
}
