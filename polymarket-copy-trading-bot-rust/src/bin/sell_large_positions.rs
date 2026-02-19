//! Sell large positions automatically
#![allow(dead_code)] // Struct fields used for JSON deserialization

use anyhow::Result;
use polymarket_copy_trading_bot_rust::config::load_env;
use polymarket_copy_trading_bot_rust::utils::{create_clob_client, fetch_data, logger::Logger};
use polymarket_client_sdk::clob::Client as ClobClient;
use polymarket_client_sdk::clob::types::{OrderType, Side};
use polymarket_client_sdk::auth::state::Authenticated;
use polymarket_client_sdk::auth::Normal;
use polymarket_client_sdk::types::Decimal;
use alloy::primitives::U256;

use chrono::DateTime;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

const SELL_PERCENTAGE: f64 = 0.8; // 80%
const MIN_POSITION_VALUE: f64 = 17.0; // Sell only positions > $17
const MIN_ORDER_SIZE_TOKENS: f64 = 1.0;

#[derive(Debug, Clone, serde::Deserialize)]
struct Position {
    asset: String,
    condition_id: String,
    size: f64,
    avg_price: f64,
    initial_value: f64,
    current_value: f64,
    cash_pnl: f64,
    percent_pnl: f64,
    total_bought: f64,
    realized_pnl: f64,
    percent_realized_pnl: f64,
    cur_price: f64,
    title: Option<String>,
    slug: Option<String>,
    outcome: Option<String>,
}

async fn sell_position(
    clob_client: &ClobClient<Authenticated<Normal>>,
    position: &Position,
    sell_size: f64,
    env: &polymarket_copy_trading_bot_rust::config::Env,
    signer: &alloy::signers::local::PrivateKeySigner,
) -> Result<bool> {
    let mut remaining = sell_size;
    let mut retry = 0u32;

    Logger::info(&format!(
        "Starting to sell {:.2} tokens ({}% of position)",
        sell_size,
        (SELL_PERCENTAGE * 100.0) as u32
    ));
    if position.asset.len() >= 20 {
        Logger::info(&format!("Token ID: {}...", &position.asset[..20]));
    }
    Logger::info(&format!("Market: {} - {}\n", position.title.as_deref().unwrap_or("Unknown"), position.outcome.as_deref().unwrap_or("Unknown")));

    while remaining > 0.0 && retry < env.retry_limit {
        // Get order book
        let book_url = format!(
            "{}/book?token_id={}",
            env.clob_http_url.trim_end_matches('/'),
            position.asset
        );
        let order_book: serde_json::Value = fetch_data(&book_url, env).await?;
        
        let bids = order_book
            .get("bids")
            .and_then(|b| b.as_array())
            .ok_or_else(|| anyhow::anyhow!("No bids in order book"))?;

        if bids.is_empty() {
            Logger::warning("No bids available in order book");
            break;
        }

        // Find best bid (maximum price)
        let best_bid = bids
            .iter()
            .filter_map(|b| {
                let price: f64 = b.get("price").and_then(|p| p.as_str()).and_then(|s| s.parse().ok())?;
                let size: f64 = b.get("size").and_then(|s| s.as_str()).and_then(|s| s.parse().ok())?;
                Some((price, size))
            })
            .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        let (best_price, best_size) = match best_bid {
            Some(bid) => bid,
            None => {
                Logger::warning("No valid bids found");
                break;
            }
        };

        Logger::info(&format!("Best bid: {} @ ${}", best_size, best_price));

        let order_amount = remaining.min(best_size);
        if order_amount < MIN_ORDER_SIZE_TOKENS {
            Logger::info("Order amount below minimum - completing sell");
            break;
        }

        // Create sell order
        let token_id = U256::from_str_radix(position.asset.trim_start_matches("0x"), 16)
            .or_else(|_| U256::from_str(&position.asset))?;
        let size_decimal = Decimal::from_str(&format!("{:.6}", order_amount))?;
        let price_decimal = Decimal::from_str(&format!("{:.6}", best_price))?;

        let exp_secs = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + 90;
        let exp = DateTime::from_timestamp(exp_secs as i64, 0)
            .ok_or_else(|| anyhow::anyhow!("Invalid timestamp"))?;

        let order = clob_client
            .limit_order()
            .token_id(token_id)
            .size(size_decimal)
            .price(price_decimal)
            .side(Side::Sell)
            .order_type(OrderType::FOK)
            .expiration(exp)
            .build()
            .await?;

        Logger::info(&format!("Selling {:.2} tokens at ${}...", order_amount, best_price));

        let signed = clob_client.sign(signer, order).await?;
        let resp = clob_client.post_order(signed).await?;

        if resp.error_msg.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
            retry = 0;
            let sold_value = order_amount * best_price;
            Logger::order_result(
                true,
                &format!(
                    "Sold {:.2} tokens at ${} (Total: ${:.2})",
                    order_amount, best_price, sold_value
                ),
            );
            remaining -= order_amount;

            if remaining > 0.0 {
                Logger::info(&format!("Remaining to sell: {:.2} tokens\n", remaining));
            }
        } else {
            retry += 1;
            let error_msg = resp.error_msg.as_deref().unwrap_or("Unknown error");
            Logger::warning(&format!(
                "Order failed (attempt {}/{}){}",
                retry,
                env.retry_limit,
                if !error_msg.is_empty() { format!(": {}", error_msg) } else { String::new() }
            ));

            if retry < env.retry_limit {
                Logger::info("Retrying...\n");
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        }
    }

    if remaining > 0.0 {
        Logger::warning(&format!(
            "Could not sell all tokens. Remaining: {:.2} tokens",
            remaining
        ));
        Ok(false)
    } else {
        Logger::success(&format!("Successfully sold {:.2} tokens!", sell_size));
        Ok(true)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Sell Large Positions Script");
    println!("═══════════════════════════════════════════════\n");

    let env = load_env()?;
    println!("📍 Wallet: {}", env.proxy_wallet);
    println!("📊 Sell percentage: {:.0}%", SELL_PERCENTAGE * 100.0);
    println!("💰 Minimum position value: ${}\n", MIN_POSITION_VALUE);

    // Create provider and client
    let (clob_client, signer) = create_clob_client(&env).await?;
    println!("✅ Connected to Polymarket\n");

    // Get all positions
    println!("📥 Fetching positions...");
    let positions_url = format!("https://data-api.polymarket.com/positions?user={}", env.proxy_wallet);
    let positions_json: serde_json::Value = fetch_data(&positions_url, &env).await?;
    let positions: Vec<Position> = positions_json
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|v| serde_json::from_value(v).ok())
        .collect();
    
    println!("Found {} position(s)\n", positions.len());

    // Filter large positions
    let mut large_positions: Vec<_> = positions
        .iter()
        .filter(|p| p.current_value > MIN_POSITION_VALUE)
        .collect();

    if large_positions.is_empty() {
        println!("✅ No positions larger than ${} found.", MIN_POSITION_VALUE);
        return Ok(());
    }

    // Sort by size
    large_positions.sort_by(|a, b| b.current_value.partial_cmp(&a.current_value).unwrap_or(std::cmp::Ordering::Equal));

    println!("🎯 Found {} large position(s):\n", large_positions.len());
    for pos in &large_positions {
        println!("  • {} [{}]", pos.title.as_deref().unwrap_or("Unknown"), pos.outcome.as_deref().unwrap_or("Unknown"));
        println!(
            "    Current: ${:.2} ({:.2} shares)",
            pos.current_value, pos.size
        );
        println!(
            "    Will sell: {:.2} shares ({:.0}%)",
            pos.size * SELL_PERCENTAGE,
            SELL_PERCENTAGE * 100.0
        );
        println!();
    }

    println!("{}\n", "━".repeat(50));

    let mut success_count = 0;
    let mut failure_count = 0;
    let mut total_sold = 0.0;

    // Sell each position
    for (i, position) in large_positions.iter().enumerate() {
        let sell_size = (position.size * SELL_PERCENTAGE).floor();

        println!("\n📦 Position {}/{}", i + 1, large_positions.len());
        println!("{}", "━".repeat(50));
        println!("Market: {}", position.title.as_deref().unwrap_or("Unknown"));
        println!("Outcome: {}", position.outcome.as_deref().unwrap_or("Unknown"));
        println!("Position size: {:.2} tokens", position.size);
        println!("Average price: ${:.4}", position.avg_price);
        println!("Current value: ${:.2}", position.current_value);
        println!("PnL: ${:.2} ({:.2}%)", position.cash_pnl, position.percent_pnl);

        if sell_size < MIN_ORDER_SIZE_TOKENS {
            println!(
                "\n⚠️  Skipping: Sell size ({:.2} tokens) is below minimum ({} token)\n",
                sell_size, MIN_ORDER_SIZE_TOKENS
            );
            failure_count += 1;
            continue;
        }

        match sell_position(&clob_client, position, sell_size, &env, &signer).await {
            Ok(true) => {
                success_count += 1;
                total_sold += sell_size;
            }
            Ok(false) => {
                failure_count += 1;
            }
            Err(e) => {
                Logger::error(&format!("Error selling position: {}", e));
                failure_count += 1;
            }
        }

        // Pause between sales
        if i < large_positions.len() - 1 {
            println!("\n⏳ Waiting 2 seconds before next sale...\n");
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }
    }

    println!("\n{}", "━".repeat(50));
    println!("📊 SUMMARY");
    println!("{}", "━".repeat(50));
    println!("✅ Successful sales: {}/{}", success_count, large_positions.len());
    println!("❌ Failed sales: {}/{}", failure_count, large_positions.len());
    println!("📦 Total tokens sold: {:.2}", total_sold);
    println!("{}\n", "━".repeat(50));

    println!("✅ Script completed!");
    Ok(())
}

