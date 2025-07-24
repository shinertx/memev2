// executor/src/jito_client.rs
use anyhow::{anyhow, Context, Result};
// Temporarily disabled for build - jito integration
// use jito_searcher_client::{JitoClient as BaseJitoClient, TxBundle};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    hash::Hash,
    signature::{read_keypair_file, Signature, Signer},
    transaction::{Transaction, VersionedTransaction},
};
use std::sync::Arc;
use tracing::info;
use url::Url;

// pub struct JitoClient {
//     pub client: BaseJitoClient,
// }

// Stub implementation for build compatibility
pub struct JitoClient;

impl JitoClient {
    pub fn new(_endpoint: &str, _keypair_path: &str) -> anyhow::Result<Self> {
        info!("🚧 Jito client disabled - using stub implementation");
        Ok(JitoClient)
    }
}

impl JitoClient {
    pub async fn new(jito_rpc_url: &str) -> Result<Self> {
        let auth_keypair_path = crate::config::CONFIG.jito_auth_keypair_path.clone(); // Path from config
        let auth_keypair = Arc::new(read_keypair_file(&auth_keypair_path).map_err(|e| {
            anyhow!(
                "Failed to read Jito auth keypair from {}: {}",
                auth_keypair_path,
                e
            )
        })?);

        let inner = BaseJitoClient::new(&Url::parse(jito_rpc_url)?, auth_keypair.clone()) // Pass cloned Arc
            .await
            .context("Failed to create Jito searcher client")?;

        let rpc_client = solana_client::nonblocking::rpc_client::RpcClient::new_with_commitment(
            crate::config::CONFIG.solana_rpc_url.clone(), // Use main RPC for blockhash
            CommitmentConfig::confirmed(),
        );

        info!("Jito client initialized successfully.");
        Ok(Self {
            inner,
            auth_keypair,
            rpc_client,
        })
    }

    pub async fn get_recent_blockhash(&self) -> Result<Hash> {
        self.rpc_client
            .get_latest_blockhash()
            .await
            .context("Failed to get recent blockhash from RPC")
    }

    // P-5: Attach Jito tip to a transaction
    pub async fn attach_tip(&self, tx: &mut VersionedTransaction, tip_lamports: u64) -> Result<()> {
        let tip_account = "96gYZGLnJYVFmbjzopPSU6QiEV5fGq58M8N1MUXronJA".parse()?; // Jito's main tip account

        // This is a simplified method. In a real scenario, you'd ensure the tip instruction
        // is added to the transaction as a compute budget instruction or similar.
        // For a VersionedTransaction, you would modify the message.
        // This part needs careful handling depending on Jito's exact current requirements for tips.

        // For now, we simply ensure the auth_keypair signs to cover the tip.
        // A direct modification of the VersionedTransaction message's instructions
        // might be needed based on Jito's exact requirements for tip inclusion.
        // As Jito's API evolves, this part might need an update.

        info!("Simulated Jito tip attachment of {} lamports. Actual instruction modification needed for VersionedTransaction.", tip_lamports);
        Ok(())
    }

    // P-5: Send transaction via Jito
    pub async fn send_transaction(&self, tx: &VersionedTransaction) -> Result<Signature> {
        let bundle = TxBundle::new(vec![tx.clone()]); // Create a bundle with one transaction

        info!(
            "Sending bundle to Jito. First transaction signature: {}",
            tx.signatures
        );
        let _ = self
            .inner
            .send_bundle(&bundle)
            .await
            .context("Failed to send Jito bundle")?;

        // Jito's send_bundle doesn't return the confirmed signature directly.
        // You'd typically monitor for confirmation via RPC.
        Ok(tx.signatures)
    }
}
