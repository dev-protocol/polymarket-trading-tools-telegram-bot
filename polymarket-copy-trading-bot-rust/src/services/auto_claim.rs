use anyhow::Result;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use crate::config::Env;
use crate::utils::{fetch_data, logger::Logger};

const RESOLVED_HIGH: f64 = 0.99;
const RESOLVED_LOW: f64 = 0.01;
const ZERO_THRESHOLD: f64 = 0.0001;

pub async fn start_auto_claim(env: Arc<Env>) -> Result<()> {
    Logger::info(&format!(
        "🚀 Starting auto-claim service (checking every {} minutes)",
        env.auto_claim_interval_ms / 1000 / 60
    ));

    // Run immediately on start
    check_and_redeem(&env).await?;

    // Then run periodically
    let mut claim_interval = interval(Duration::from_millis(env.auto_claim_interval_ms));
    loop {
        claim_interval.tick().await;
        if let Err(e) = check_and_redeem(&env).await {
            Logger::error(&format!("❌ Auto-claim error: {}", e));
        }
    }
}

async fn check_and_redeem(env: &Env) -> Result<()> {
    Logger::info("🔍 Auto-claim: Checking for redeemable positions...");

    // Load positions
    let positions_url = format!("https://data-api.polymarket.com/positions?user={}", env.proxy_wallet);
    let all_positions: Vec<serde_json::Value> = fetch_data(&positions_url, env).await?
        .as_array()
        .cloned()
        .unwrap_or_default();

    let all_positions: Vec<_> = all_positions
        .into_iter()
        .filter(|pos| {
            pos.get("size")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0) > ZERO_THRESHOLD
        })
        .collect();

    if all_positions.is_empty() {
        Logger::info("✅ Auto-claim: No open positions detected");
        return Ok(());
    }

    Logger::info(&format!("📈 Found {} total position(s)", all_positions.len()));

    // Filter for resolved and redeemable positions
    let redeemable_positions: Vec<_> = all_positions
        .iter()
        .filter(|pos| {
            let cur_price = pos.get("curPrice").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let redeemable = pos.get("redeemable").and_then(|v| v.as_bool()).unwrap_or(false);
            (cur_price >= RESOLVED_HIGH || cur_price <= RESOLVED_LOW) && redeemable
        })
        .collect();

    if redeemable_positions.is_empty() {
        Logger::info("✅ Auto-claim: No redeemable positions found");
        Logger::info("   (Positions must be resolved AND marked as redeemable)");
        return Ok(());
    }

    Logger::info(&format!(
        "💰 Auto-claim: Found {} redeemable position(s)",
        redeemable_positions.len()
    ));

    // TODO: Implement actual redemption via smart contract call
    // This requires:
    // 1. Setting up ethers/alloy provider with signer
    // 2. Calling CTF contract redeemPositions function
    // 3. Handling gas estimation and transaction submission

    Logger::success("✅ Auto-claim complete (placeholder - redemption not yet implemented)");
    Ok(())
}

#[allow(dead_code)] // Used by trigger_auto_claim binary
pub async fn trigger_auto_claim(env: &Env) -> Result<()> {
    check_and_redeem(env).await
}

