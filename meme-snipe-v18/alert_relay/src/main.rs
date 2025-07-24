// alert_relay/src/main.rs
use anyhow::*;
use redis::AsyncCommands;
use std::env;
use tracing::{info, warn, error};
use chrono::Utc;

#[derive(serde::Deserialize, serde::Serialize)]
struct Alert {
    message: String,
    timestamp: String,
    service: String,
    level: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    
    let redis_url = env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://redis:6379".to_string());
    let telegram_bot_token = env::var("TELEGRAM_BOT_TOKEN").ok();
    let telegram_chat_id = env::var("TELEGRAM_CHAT_ID").ok();
    let discord_webhook_url = env::var("DISCORD_WEBHOOK_URL").ok();
    
    info!("🚨 Starting Alert Relay...");
    info!("📱 Telegram: {}", if telegram_bot_token.is_some() { "Enabled" } else { "Disabled" });
    info!("💬 Discord: {}", if discord_webhook_url.is_some() { "Enabled" } else { "Disabled" });
    
    let client = redis::Client::open(redis_url)?;
    let mut conn = client.get_async_connection().await?;
    
    // Subscribe to alert channels
    let mut pubsub = conn.into_pubsub();
    pubsub.subscribe("alerts").await?;
    pubsub.subscribe("trading_alerts").await?;
    pubsub.subscribe("system_alerts").await?;
    pubsub.subscribe("kill_switch_channel").await?;
    
    info!("📡 Listening for alerts...");
    
    loop {
        match pubsub.get_message().await {
            Ok(msg) => {
                let channel: String = msg.get_channel_name().to_string();
                let payload: String = msg.get_payload()?;
                
                info!("📨 Alert from {}: {}", channel, payload);
                
                let alert = Alert {
                    message: payload.clone(),
                    timestamp: Utc::now().to_rfc3339(),
                    service: channel.clone(),
                    level: determine_alert_level(&channel, &payload),
                };
                
                // Send to Telegram
                if let (Some(ref token), Some(ref chat_id)) = (&telegram_bot_token, &telegram_chat_id) {
                    if let Err(e) = send_telegram_alert(token, chat_id, &alert).await {
                        error!("Failed to send Telegram alert: {}", e);
                    }
                }
                
                // Send to Discord
                if let Some(ref webhook_url) = discord_webhook_url {
                    if let Err(e) = send_discord_alert(webhook_url, &alert).await {
                        error!("Failed to send Discord alert: {}", e);
                    }
                }
            }
            Err(e) => {
                error!("Redis subscription error: {}", e);
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        }
    }
}

fn determine_alert_level(channel: &str, message: &str) -> String {
    if channel == "kill_switch_channel" || message.contains("🚨") {
        "CRITICAL".to_string()
    } else if message.contains("⚠️") || channel.contains("system") {
        "WARNING".to_string()
    } else {
        "INFO".to_string()
    }
}

async fn send_telegram_alert(bot_token: &str, chat_id: &str, alert: &Alert) -> Result<()> {
    let emoji = match alert.level.as_str() {
        "CRITICAL" => "🚨",
        "WARNING" => "⚠️",
        _ => "ℹ️",
    };
    
    let formatted_message = format!(
        "{} *MemeSnipe v18*\n\n*{}*\n\n`{}`\n\n_{}_",
        emoji,
        alert.level,
        alert.message,
        alert.timestamp
    );
    
    let url = format!("https://api.telegram.org/bot{}/sendMessage", bot_token);
    let payload = serde_json::json!({
        "chat_id": chat_id,
        "text": formatted_message,
        "parse_mode": "Markdown",
        "disable_web_page_preview": true
    });
    
    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .json(&payload)
        .send()
        .await?;
    
    if response.status().is_success() {
        info!("📱 Telegram alert sent successfully");
    } else {
        warn!("📱 Telegram alert failed: {}", response.status());
    }
    
    Ok(())
}

async fn send_discord_alert(webhook_url: &str, alert: &Alert) -> Result<()> {
    let color = match alert.level.as_str() {
        "CRITICAL" => 0xFF0000, // Red
        "WARNING" => 0xFFA500,  // Orange
        _ => 0x0099FF,          // Blue
    };
    
    let payload = serde_json::json!({
        "embeds": [{
            "title": format!("MemeSnipe v18 - {}", alert.level),
            "description": alert.message,
            "color": color,
            "timestamp": alert.timestamp,
            "footer": {
                "text": format!("From: {}", alert.service)
            }
        }]
    });
    
    let client = reqwest::Client::new();
    let response = client
        .post(webhook_url)
        .json(&payload)
        .send()
        .await?;
    
    if response.status().is_success() {
        info!("💬 Discord alert sent successfully");
    } else {
        warn!("💬 Discord alert failed: {}", response.status());
    }
    
    Ok(())
}
