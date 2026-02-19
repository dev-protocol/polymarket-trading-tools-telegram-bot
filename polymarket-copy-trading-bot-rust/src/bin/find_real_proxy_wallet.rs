//! Find real proxy wallet
#![allow(dead_code)] // Struct fields used for JSON deserialization

use anyhow::Result;
use alloy::signers::local::PrivateKeySigner;

use std::str::FromStr;
use polymarket_copy_trading_bot_rust::config::load_env;
use polymarket_copy_trading_bot_rust::utils::fetch_data;

#[tokio::main]
async fn main() -> Result<()> {
    println!("\n🔍 FINDING REAL PROXY WALLET\n");
    println!("{}\n", "━".repeat(65));

    let env = load_env()?;

    // Get EOA address from private key
    let private_key = if env.private_key.starts_with("0x") {
        env.private_key.clone()
    } else {
        format!("0x{}", env.private_key)
    };
    let signer = PrivateKeySigner::from_str(&private_key)?;
    let eoa_address = signer.address().to_string();

    println!("📋 EOA address (from private key):\n");
    println!("   {}\n", eoa_address);

    // 1. Check username API
    println!("{}\n", "━".repeat(65));
    println!("📋 STEP 1: Check username via API\n");

    let user_profile_url = format!("https://data-api.polymarket.com/users/{}", eoa_address);
    match fetch_data(&user_profile_url, &env).await {
        Ok(profile) => {
            println!("   Profile data: {}", serde_json::to_string_pretty(&profile)?);
            println!();
        }
        Err(_) => {
            println!("   ⚠️  Failed to get profile via /users\n");
        }
    }

    // 2. Check activity to find proxyWallet
    println!("{}\n", "━".repeat(65));
    println!("📋 STEP 2: Analyze transactions on Polymarket\n");

    let activities_url = format!("https://data-api.polymarket.com/activity?user={}&type=TRADE", eoa_address);
    match fetch_data(&activities_url, &env).await {
        Ok(activities_json) => {
            let activities: Vec<serde_json::Value> = activities_json
                .as_array()
                .cloned()
                .unwrap_or_default();

            if !activities.is_empty() {
                println!("   ✅ Found {} trades\n", activities.len());
                
                if let Some(first_trade) = activities.first() {
                    if let Some(proxy_wallet) = first_trade.get("proxyWallet").and_then(|v| v.as_str()) {
                        println!("   🎯 PROXY WALLET FOUND!\n");
                        println!("   Proxy Wallet: {}\n", proxy_wallet);
                        println!("   💡 Update your .env file:\n");
                        println!("   PROXY_WALLET={}\n", proxy_wallet);
                    } else {
                        println!("   ⚠️  No proxyWallet field in trades\n");
                    }
                }
            } else {
                println!("   ❌ No trades found for EOA address\n");
            }
        }
        Err(e) => {
            println!("   ⚠️  Failed to check transactions: {}\n", e);
        }
    }

    // 3. Check positions on EOA
    println!("{}\n", "━".repeat(65));
    println!("📋 STEP 3: Check positions on EOA\n");

    let positions_url = format!("https://data-api.polymarket.com/positions?user={}", eoa_address);
    match fetch_data(&positions_url, &env).await {
        Ok(positions_json) => {
            let positions: Vec<serde_json::Value> = positions_json
                .as_array()
                .cloned()
                .unwrap_or_default();
            println!("   Positions on EOA: {}\n", positions.len());
        }
        Err(_) => {
            println!("   ⚠️  Failed to get positions\n");
        }
    }

    println!("{}\n", "━".repeat(65));
    println!("💡 TIPS:\n");
    println!("   • If proxyWallet found in trades, use that address");
    println!("   • If no trades found, you may need to make a trade first");
    println!("   • Check Polymarket profile: https://polymarket.com/profile/{}", eoa_address);
    println!("   • Check Polygon explorer: https://polygonscan.com/address/{}\n", eoa_address);

    Ok(())
}

