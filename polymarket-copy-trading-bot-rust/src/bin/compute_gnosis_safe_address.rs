//! Compute Gnosis Safe proxy address
#![allow(dead_code)] // Struct fields used for JSON deserialization

use anyhow::Result;
use alloy::signers::local::PrivateKeySigner;

use std::str::FromStr;
use polymarket_copy_trading_bot_rust::config::load_env;
use polymarket_copy_trading_bot_rust::utils::fetch_data;

#[allow(dead_code)]
const GNOSIS_SAFE_PROXY_FACTORY: &str = "0xaacfeea03eb1561c4e67d661e40682bd20e3541b";
#[allow(dead_code)]
const POLYMARKET_PROXY_FACTORY: &str = "0xab45c5a4b0c941a2f231c04c3f49182e1a254052";

#[tokio::main]
async fn main() -> Result<()> {
    println!("\n🔍 COMPUTING GNOSIS SAFE PROXY ADDRESS\n");
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

    println!("{}\n", "━".repeat(65));
    println!("📋 Searching for Gnosis Safe Proxy via events\n");
    println!("   ⚠️  This requires querying blockchain events which can be slow");
    println!("   💡 Alternative: Check your trades for proxyWallet field\n");

    // Check activity to find proxyWallet
    println!("{}\n", "━".repeat(65));
    println!("📋 STEP 1: Check trades for proxyWallet\n");

    let activities_url = format!("https://data-api.polymarket.com/activity?user={}&type=TRADE", eoa_address);
    match fetch_data(&activities_url, &env).await {
        Ok(activities_json) => {
            let activities: Vec<serde_json::Value> = activities_json
                .as_array()
                .cloned()
                .unwrap_or_default();

            if !activities.is_empty() {
                if let Some(first_trade) = activities.first() {
                    if let Some(proxy_wallet) = first_trade.get("proxyWallet").and_then(|v| v.as_str()) {
                        println!("   🎯 FOUND PROXY WALLET!\n");
                        println!("   Proxy address: {}\n", proxy_wallet);

                        // Check positions
                        let positions_url = format!("https://data-api.polymarket.com/positions?user={}", proxy_wallet);
                        match fetch_data(&positions_url, &env).await {
                            Ok(positions_json) => {
                                let positions: Vec<serde_json::Value> = positions_json
                                    .as_array()
                                    .cloned()
                                    .unwrap_or_default();
                                println!("   Positions on Proxy: {}\n", positions.len());

                                if !positions.is_empty() {
                                    println!("{}\n", "━".repeat(65));
                                    println!("✅ SOLUTION FOUND!\n");
                                    println!("{}\n", "━".repeat(65));
                                    println!("Update .env file:\n");
                                    println!("PROXY_WALLET={}\n", proxy_wallet);
                                    println!("{}\n", "━".repeat(65));
                                    return Ok(());
                                }
                            }
                            Err(_) => {}
                        }
                    }
                }
            }
        }
        Err(_) => {}
    }

    println!("{}\n", "━".repeat(65));
    println!("💡 ALTERNATIVE METHODS:\n");
    println!("   1. Check your Polymarket profile for the proxy wallet address");
    println!("   2. Use find_real_proxy_wallet script");
    println!("   3. Check Polygon explorer for contract creation events");
    println!("   4. Contact Polymarket support if needed\n");

    println!("{}\n", "━".repeat(65));

    Ok(())
}

