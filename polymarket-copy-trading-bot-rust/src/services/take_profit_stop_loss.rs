use anyhow::Result;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use crate::config::Env;
use crate::utils::{fetch_data, logger::Logger};
use polymarket_client_sdk::clob::Client as ClobClient;
use polymarket_client_sdk::auth::state::Authenticated;
use polymarket_client_sdk::auth::Normal;

pub async fn start_take_profit_stop_loss(
    _clob_client: Arc<ClobClient<Authenticated<Normal>>>,
    env: Arc<Env>,
) -> Result<()> {
    // Only start if at least one TP/SL is configured
    if env.take_profit_percent.is_none() && env.stop_loss_percent.is_none() {
        Logger::info("Take Profit / Stop Loss monitor disabled (no TP/SL configured)");
        return Ok(());
    }

    Logger::success("Take Profit / Stop Loss monitor started");
    if let Some(tp) = env.take_profit_percent {
        Logger::info(&format!("Take Profit: {}%", tp));
    } else {
        Logger::info("Take Profit: Disabled");
    }
    if let Some(sl) = env.stop_loss_percent {
        Logger::info(&format!("Stop Loss: {}%", sl));
    } else {
        Logger::info("Stop Loss: Disabled");
    }
    Logger::info(&format!("Check Interval: {}ms", env.tp_sl_check_interval_ms));

    // Run initial check
    monitor_positions(&env).await?;

    // Set up interval
    let mut monitor_interval = interval(Duration::from_millis(env.tp_sl_check_interval_ms));
    loop {
        monitor_interval.tick().await;
        if let Err(e) = monitor_positions(&env).await {
            Logger::error(&format!("Error monitoring positions for TP/SL: {}", e));
        }
    }
}

async fn monitor_positions(env: &Env) -> Result<()> {
    let positions_url = format!("https://data-api.polymarket.com/positions?user={}", env.proxy_wallet);
    let positions: Vec<serde_json::Value> = fetch_data(&positions_url, env).await?
        .as_array()
        .cloned()
        .unwrap_or_default();

    if positions.is_empty() {
        return Ok(());
    }

    // Check each position
    for position in &positions {
        check_position(position, env).await?;
    }

    Ok(())
}

async fn check_position(position: &serde_json::Value, env: &Env) -> Result<()> {
    let avg_price = position.get("avgPrice").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let current_price = position.get("curPrice").and_then(|v| v.as_f64()).unwrap_or(0.0);

    if avg_price <= 0.0 || current_price <= 0.0 {
        return Ok(());
    }

    // Calculate price change percentage
    let price_change_percent = ((current_price - avg_price) / avg_price) * 100.0;

    // Check Take Profit (if configured)
    if let Some(tp_percent) = env.take_profit_percent {
        if price_change_percent >= tp_percent {
            Logger::header("🎯 TAKE PROFIT TRIGGERED");
            Logger::info(&format!(
                "Position: {}",
                position
                    .get("slug")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown")
            ));
            Logger::info(&format!("Entry Price: ${:.4}", avg_price));
            Logger::info(&format!("Current Price: ${:.4}", current_price));
            Logger::info(&format!(
                "Profit: +{:.2}% (Target: +{}%)",
                price_change_percent, tp_percent
            ));
            Logger::info("Selling position immediately...");

            // TODO: Implement actual sell via CLOB client
            return Ok(());
        }
    }

    // Check Stop Loss (if configured)
    if let Some(sl_percent) = env.stop_loss_percent {
        if price_change_percent <= -sl_percent {
            Logger::header("🛑 STOP LOSS TRIGGERED");
            Logger::info(&format!(
                "Position: {}",
                position
                    .get("slug")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown")
            ));
            Logger::info(&format!("Entry Price: ${:.4}", avg_price));
            Logger::info(&format!("Current Price: ${:.4}", current_price));
            Logger::info(&format!(
                "Loss: {:.2}% (Limit: -{}%)",
                price_change_percent, sl_percent
            ));
            Logger::info("Selling position immediately...");

            // TODO: Implement actual sell via CLOB client
            return Ok(());
        }
    }

    Ok(())
}

