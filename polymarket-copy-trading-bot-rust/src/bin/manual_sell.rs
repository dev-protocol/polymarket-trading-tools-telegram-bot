//! Manual sell script - sell positions by market search
#![allow(dead_code)] // Struct fields used for JSON deserialization

use anyhow::Result;
use std::io::{self, Write};
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

const MIN_ORDER_SIZE_TOKENS: f64 = 1.0;

#[derive(Debug, serde::Deserialize)]
struct Position {
    asset: String,
    condition_id: String,
    size: f64,
    avg_price: f64,
    current_value: f64,
    title: String,
    outcome: String,
}

fn find_matching_position<'a>(positions: &'a [Position], search_query: &str) -> Option<&'a Position> {
    let query_lower = search_query.to_lowercase();
    
    // Try exact match first
    if let Some(pos) = positions.iter().find(|p| p.title.to_lowercase() == query_lower) {
        return Some(pos);
    }
    
    // Try contains match
    if let Some(pos) = positions.iter().find(|p| p.title.to_lowercase().contains(&query_lower)) {
        return Some(pos);
    }
    
    // Try matching individual words
    let query_words: Vec<&str> = query_lower.split_whitespace().filter(|w| w.len() > 2).collect();
    if !query_words.is_empty() {
        if let Some(pos) = positions.iter().find(|p| {
            let title_lower = p.title.to_lowercase();
            query_words.iter().any(|word| title_lower.contains(word))
        }) {
            return Some(pos);
        }
    }
    
    None
}

async fn sell_position(
    clob_client: &ClobClient<Authenticated<Normal>>,
    position: &Position,
    sell_size: f64,
    env: &polymarket_copy_trading_bot_rust::config::Env,
    signer: &alloy::signers::local::PrivateKeySigner,
) -> Result<()> {
    let mut remaining = sell_size;
    let mut retry = 0u32;

    Logger::info(&format!(
        "Starting to sell {:.2} tokens",
        sell_size
    ));
    Logger::info(&format!("Token ID: {}", position.asset));
    Logger::info(&format!("Market: {} - {}\n", position.title, position.outcome));

    while remaining > 0.0 && retry < env.retry_limit {
        // Get order book
        let book_url = format!(
            "{}/book?token_id={}",
            env.clob_http_url.trim_end_matches('/'),
            position.asset
        );
        let order_book: serde_json::Value = fetch_data(&book_url, &env).await?;
        
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
    } else {
        Logger::success(&format!("Successfully sold {:.2} tokens!", sell_size));
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Manual Sell Script");
    println!("═══════════════════════════════════════════════\n");

    let env = load_env()?;
    println!("📍 Wallet: {}", env.proxy_wallet);

    // Get search query from user
    print!("🔍 Enter market search query: ");
    io::stdout().flush()?;
    let mut search_query = String::new();
    io::stdin().read_line(&mut search_query)?;
    let search_query = search_query.trim();

    // Get sell percentage
    print!("📊 Enter sell percentage (0.0-1.0, e.g., 0.7 for 70%): ");
    io::stdout().flush()?;
    let mut sell_percent_str = String::new();
    io::stdin().read_line(&mut sell_percent_str)?;
    let sell_percentage: f64 = sell_percent_str.trim().parse()?;
    
    if sell_percentage <= 0.0 || sell_percentage > 1.0 {
        eprintln!("❌ Invalid percentage. Must be between 0.0 and 1.0");
        std::process::exit(1);
    }

    println!("📊 Sell percentage: {:.0}%\n", sell_percentage * 100.0);

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

    // Find matching position
    let position = find_matching_position(&positions, search_query);

    if let Some(pos) = position {
        println!("✅ Position found!");
        println!("📌 Market: {}", pos.title);
        println!("📌 Outcome: {}", pos.outcome);
        println!("📌 Position size: {:.2} tokens", pos.size);
        println!("📌 Average price: ${:.4}", pos.avg_price);
        println!("📌 Current value: ${:.2}", pos.current_value);

        let sell_size = pos.size * sell_percentage;

        if sell_size < MIN_ORDER_SIZE_TOKENS {
            eprintln!(
                "\n❌ Sell size ({:.2} tokens) is below minimum ({} token)",
                sell_size, MIN_ORDER_SIZE_TOKENS
            );
            eprintln!("Please increase your position or adjust sell percentage");
            std::process::exit(1);
        }

        // Sell position
        sell_position(&clob_client, pos, sell_size, &env, &signer).await?;

        println!("\n✅ Script completed!");
    } else {
        eprintln!("❌ Position \"{}\" not found!", search_query);
        println!("\nAvailable positions:");
        for (idx, pos) in positions.iter().enumerate() {
            println!(
                "{}. {} - {} ({:.2} tokens)",
                idx + 1, pos.title, pos.outcome, pos.size
            );
        }
        println!("\n💡 Tip: Use a partial match from the position title above.");
        std::process::exit(1);
    }

    Ok(())
}

