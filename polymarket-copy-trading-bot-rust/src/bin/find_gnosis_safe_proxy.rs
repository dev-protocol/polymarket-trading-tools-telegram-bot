//! Find Gnosis Safe proxy wallet
#![allow(dead_code)] // Struct fields used for JSON deserialization

use anyhow::Result;
use alloy::signers::local::PrivateKeySigner;

use std::str::FromStr;
use polymarket_copy_trading_bot_rust::config::load_env;
use polymarket_copy_trading_bot_rust::utils::fetch_data;

#[tokio::main]
async fn main() -> Result<()> {
    println!("\n🔍 FINDING GNOSIS SAFE PROXY WALLET\n");
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

    println!("📋 STEP 1: Your EOA address (from private key)\n");
    println!("   {}\n", eoa_address);

    // 2. Look for all positions on EOA
    println!("{}\n", "━".repeat(65));
    println!("📋 STEP 2: Positions on EOA address\n");

    let eoa_positions_url = format!("https://data-api.polymarket.com/positions?user={}", eoa_address);
    match fetch_data(&eoa_positions_url, &env).await {
        Ok(positions_json) => {
            let positions: Vec<serde_json::Value> = positions_json
                .as_array()
                .cloned()
                .unwrap_or_default();
            println!("   Positions: {}\n", positions.len());

            if !positions.is_empty() {
                println!("   ✅ There are positions on EOA!\n");
            }
        }
        Err(_) => {
            println!("   ❌ Failed to get positions\n");
        }
    }

    // 3. Look for EOA transactions to find proxy
    println!("{}\n", "━".repeat(65));
    println!("📋 STEP 3: Finding Gnosis Safe Proxy via transactions\n");

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
                        println!("   Proxy Wallet: {}\n", proxy_wallet);

                        // Check if it's a contract
                        let code_body = serde_json::json!({
                            "jsonrpc": "2.0",
                            "method": "eth_getCode",
                            "params": [proxy_wallet, "latest"],
                            "id": 1
                        });
                        let client = reqwest::Client::new();
                        let code_resp = client.post(&env.rpc_url).json(&code_body).send().await?;
                        let code_json: serde_json::Value = code_resp.json().await?;
                        let code = code_json.get("result").and_then(|v| v.as_str()).unwrap_or("0x");
                        let is_contract = code != "0x" && code.chars().any(|c| c != '0');

                        if is_contract {
                            println!("   ✅ This is a smart contract (likely Gnosis Safe)\n");
                        }

                        // Check positions on proxy
                        let proxy_positions_url = format!("https://data-api.polymarket.com/positions?user={}", proxy_wallet);
                        match fetch_data(&proxy_positions_url, &env).await {
                            Ok(proxy_positions_json) => {
                                let proxy_positions: Vec<serde_json::Value> = proxy_positions_json
                                    .as_array()
                                    .cloned()
                                    .unwrap_or_default();
                                println!("   Positions on Proxy: {}\n", proxy_positions.len());

                                if !proxy_positions.is_empty() {
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
        Err(_) => {
            println!("   ⚠️  Failed to check transactions\n");
        }
    }

    println!("{}\n", "━".repeat(65));
    println!("💡 TIPS:\n");
    println!("   • Check your Polymarket profile for the proxy wallet");
    println!("   • Use find_real_proxy_wallet script for more detailed search");
    println!("   • Check Polygon explorer for contract creation events\n");

    Ok(())
}

