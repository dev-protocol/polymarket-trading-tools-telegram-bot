//! Close stale positions (tracked traders already exited)
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
use std::collections::HashSet;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

const MIN_SELL_TOKENS: f64 = 1.0;
const ZERO_THRESHOLD: f64 = 0.0001;

#[derive(Debug, Clone, serde::Deserialize)]
struct Position {
    asset: String,
    condition_id: String,
    size: f64,
    avg_price: f64,
    current_value: f64,
    cur_price: f64,
    title: Option<String>,
    outcome: Option<String>,
    slug: Option<String>,
    redeemable: Option<bool>,
}

async fn sell_entire_position(
    clob_client: &ClobClient<Authenticated<Normal>>,
    position: &Position,
    env: &polymarket_copy_trading_bot_rust::config::Env,
    signer: &alloy::signers::local::PrivateKeySigner,
) -> Result<(f64, f64, f64)> {
    let mut remaining = position.size;
    let mut attempts = 0u32;
    let mut sold_tokens = 0.0;
    let mut proceeds_usd = 0.0;

    if remaining < MIN_SELL_TOKENS {
        return Ok((0.0, 0.0, remaining));
    }

    while remaining >= MIN_SELL_TOKENS && attempts < env.retry_limit {
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
            break;
        }

        let best_bid = bids
            .iter()
            .filter_map(|b| {
                let price: f64 = b.get("price").and_then(|p| p.as_str()).and_then(|s| s.parse().ok())?;
                let size: f64 = b.get("size").and_then(|s| s.as_str()).and_then(|s| s.parse().ok())?;
                Some((price, size))
            })
            .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        let (bid_price, bid_size) = match best_bid {
            Some(bid) => bid,
            None => break,
        };

        if bid_size < MIN_SELL_TOKENS {
            break;
        }

        let sell_amount = remaining.min(bid_size);

        if sell_amount < MIN_SELL_TOKENS {
            break;
        }

        let token_id = U256::from_str_radix(position.asset.trim_start_matches("0x"), 16)
            .or_else(|_| U256::from_str(&position.asset))?;
        let size_decimal = Decimal::from_str(&format!("{:.6}", sell_amount))?;
        let price_decimal = Decimal::from_str(&format!("{:.6}", bid_price))?;

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

        match clob_client.sign(signer, order).await {
            Ok(signed) => {
                match clob_client.post_order(signed).await {
                    Ok(resp) => {
                        if resp.error_msg.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
                            let trade_value = sell_amount * bid_price;
                            sold_tokens += sell_amount;
                            proceeds_usd += trade_value;
                            remaining -= sell_amount;
                            attempts = 0;
                        } else {
                            attempts += 1;
                        }
                    }
                    Err(_) => {
                        attempts += 1;
                    }
                }
            }
            Err(_) => {
                attempts += 1;
            }
        }
    }

    Ok((sold_tokens, proceeds_usd, remaining))
}

async fn build_tracked_set(env: &polymarket_copy_trading_bot_rust::config::Env) -> Result<HashSet<String>> {
    let mut tracked = HashSet::new();

    for user in &env.user_addresses {
        let positions_url = format!("https://data-api.polymarket.com/positions?user={}", user);
        match fetch_data(&positions_url, env).await {
            Ok(positions_json) => {
                let positions: Vec<Position> = positions_json
                    .as_array()
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|v| serde_json::from_value(v).ok())
                    .collect();
                
                for pos in positions {
                    if pos.size > ZERO_THRESHOLD {
                        tracked.insert(format!("{}:{}", pos.condition_id, pos.asset));
                    }
                }
            }
            Err(e) => {
                Logger::warning(&format!("Failed to load positions for {}: {}", user, e));
            }
        }
    }

    Ok(tracked)
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Closing stale positions (tracked traders already exited)");
    println!("════════════════════════════════════════════════════");
    
    let env = load_env()?;
    println!("Wallet: {}", env.proxy_wallet);

    let (clob_client, signer) = create_clob_client(&env).await?;
    println!("✅ Connected to Polymarket CLOB");

    let my_positions_json = fetch_data(&format!("https://data-api.polymarket.com/positions?user={}", env.proxy_wallet), &env).await?;
    let tracked_set = build_tracked_set(&env).await?;

    let my_positions: Vec<Position> = my_positions_json
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|v| serde_json::from_value::<Position>(v).ok())
        .filter(|pos: &Position| pos.size > ZERO_THRESHOLD)
        .collect();

    if my_positions.is_empty() {
        println!("\n🎉 No open positions detected for proxy wallet.");
        return Ok(());
    }

    let stale_positions: Vec<_> = my_positions
        .iter()
        .filter(|pos| !tracked_set.contains(&format!("{}:{}", pos.condition_id, pos.asset)))
        .collect();

    if stale_positions.is_empty() {
        println!("\n✅ All positions still held by tracked traders. Nothing to close.");
        return Ok(());
    }

    println!("\nFound {} stale position(s) to unwind.", stale_positions.len());

    let mut total_tokens = 0.0;
    let mut total_proceeds = 0.0;

    for (i, position) in stale_positions.iter().enumerate() {
        println!("\n{}/{} ▶ {}", i + 1, stale_positions.len(), position.title.as_deref().or(position.slug.as_deref()).unwrap_or(&position.asset));
        if let Some(outcome) = &position.outcome {
            println!("   Outcome: {}", outcome);
        }
        println!("   Size: {:.2} tokens @ avg ${:.3}", position.size, position.avg_price);
        println!("   Est. value: ${:.2} (cur price ${:.3})", position.current_value, position.cur_price);
        if position.redeemable == Some(true) {
            println!("   ℹ️  Market is redeemable — consider redeeming if value stays flat at $0.");
        }

        match sell_entire_position(&clob_client, position, &env, &signer).await {
            Ok((sold, proceeds, _remaining)) => {
                total_tokens += sold;
                total_proceeds += proceeds;
            }
            Err(e) => {
                Logger::error(&format!("Failed to close position: {}", e));
            }
        }
    }

    println!("\n════════════════════════════════════════════════════");
    println!("✅ Close-out summary");
    println!("Markets touched: {}", stale_positions.len());
    println!("Tokens sold: {:.2}", total_tokens);
    println!("USDC realized (approx.): ${:.2}", total_proceeds);
    println!("════════════════════════════════════════════════════\n");

    Ok(())
}

