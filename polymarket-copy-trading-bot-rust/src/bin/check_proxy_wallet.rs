//! Check proxy wallet and main wallet
#![allow(dead_code)] // Struct fields used for JSON deserialization

use anyhow::Result;
use alloy::signers::local::PrivateKeySigner;

use std::str::FromStr;
use polymarket_copy_trading_bot_rust::config::load_env;
use polymarket_copy_trading_bot_rust::utils::fetch_data;

#[derive(Debug, serde::Deserialize)]
struct Activity {
    proxy_wallet: Option<String>,
    timestamp: i64,
    condition_id: String,
    #[serde(rename = "type")]
    activity_type: String,
    size: f64,
    usdc_size: f64,
    transaction_hash: String,
    price: f64,
    asset: String,
    side: String,
    title: Option<String>,
}

async fn is_contract_address(rpc_url: &str, address: &str) -> Result<bool> {
    let addr_trimmed = address.trim().trim_start_matches("0x");
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_getCode",
        "params": [format!("0x{}", addr_trimmed), "latest"],
        "id": 1
    });
    let client = reqwest::Client::new();
    let resp = client.post(rpc_url).json(&body).send().await?;
    let json: serde_json::Value = resp.json().await?;
    let result = json.get("result").and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("No result in RPC response"))?;
    let code = result.trim_start_matches("0x");
    Ok(!code.is_empty() && code.chars().any(|c| c != '0'))
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 CHECKING PROXY WALLET AND MAIN WALLET\n");
    println!("{}\n", "━".repeat(65));

    let env = load_env()?;
    
    // Get EOA (main wallet) from private key
    let private_key = if env.private_key.starts_with("0x") {
        env.private_key.clone()
    } else {
        format!("0x{}", env.private_key)
    };
    let signer = PrivateKeySigner::from_str(&private_key)?;
    let eoa_address = signer.address().to_string();

    println!("📍 YOUR ADDRESSES:\n");
    println!("   EOA (Main wallet):  {}", eoa_address);
    println!("   Proxy Wallet (Contract): {}\n", env.proxy_wallet);

    println!("{}\n", "━".repeat(65));

    // Check activity on EOA
    println!("🔎 CHECKING ACTIVITY ON MAIN WALLET (EOA):\n");
    let eoa_activity_url = format!("https://data-api.polymarket.com/activity?user={}&type=TRADE", eoa_address);
    let eoa_activities_json: serde_json::Value = fetch_data(&eoa_activity_url, &env).await?;
    let eoa_activities: Vec<Activity> = eoa_activities_json
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|v| serde_json::from_value(v).ok())
        .collect();

    println!("   Address: {}", eoa_address);
    println!("   Trades: {}", eoa_activities.len());
    println!("   Profile: https://polymarket.com/profile/{}\n", eoa_address);

    if !eoa_activities.is_empty() {
        let buy_trades: Vec<_> = eoa_activities.iter().filter(|a| a.side == "BUY").collect();
        let sell_trades: Vec<_> = eoa_activities.iter().filter(|a| a.side == "SELL").collect();
        let total_buy_volume: f64 = buy_trades.iter().map(|t| t.usdc_size).sum();
        let total_sell_volume: f64 = sell_trades.iter().map(|t| t.usdc_size).sum();

        println!("   📊 EOA Statistics:");
        println!("      • Buys: {} (${:.2})", buy_trades.len(), total_buy_volume);
        println!("      • Sells: {} (${:.2})", sell_trades.len(), total_sell_volume);
        println!("      • Volume: ${:.2}\n", total_buy_volume + total_sell_volume);

        println!("   📝 Last 3 trades:");
        for (idx, trade) in eoa_activities.iter().take(3).enumerate() {
            let date = chrono::DateTime::from_timestamp(trade.timestamp, 0)
                .map(|dt| dt.format("%Y-%m-%d").to_string())
                .unwrap_or_else(|| "Unknown".to_string());
            println!("      {}. {} - {}", idx + 1, trade.side, trade.title.as_deref().unwrap_or("Unknown"));
            println!("         ${:.2} @ {}", trade.usdc_size, date);
        }
        println!();
    } else {
        println!("   ❌ No trades found on main wallet\n");
    }

    println!("{}\n", "━".repeat(65));

    // Check activity on Proxy Wallet
    println!("🔎 CHECKING ACTIVITY ON PROXY WALLET (CONTRACT):\n");
    let proxy_activity_url = format!("https://data-api.polymarket.com/activity?user={}&type=TRADE", env.proxy_wallet);
    let proxy_activities_json: serde_json::Value = fetch_data(&proxy_activity_url, &env).await?;
    let proxy_activities: Vec<Activity> = proxy_activities_json
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|v| serde_json::from_value(v).ok())
        .collect();

    println!("   Address: {}", env.proxy_wallet);
    println!("   Trades: {}", proxy_activities.len());
    println!("   Profile: https://polymarket.com/profile/{}\n", env.proxy_wallet);

    if !proxy_activities.is_empty() {
        let buy_trades: Vec<_> = proxy_activities.iter().filter(|a| a.side == "BUY").collect();
        let sell_trades: Vec<_> = proxy_activities.iter().filter(|a| a.side == "SELL").collect();
        let total_buy_volume: f64 = buy_trades.iter().map(|t| t.usdc_size).sum();
        let total_sell_volume: f64 = sell_trades.iter().map(|t| t.usdc_size).sum();

        println!("   📊 Proxy Wallet Statistics:");
        println!("      • Buys: {} (${:.2})", buy_trades.len(), total_buy_volume);
        println!("      • Sells: {} (${:.2})", sell_trades.len(), total_sell_volume);
        println!("      • Volume: ${:.2}\n", total_buy_volume + total_sell_volume);

        println!("   📝 Last 3 trades:");
        for (idx, trade) in proxy_activities.iter().take(3).enumerate() {
            let date = chrono::DateTime::from_timestamp(trade.timestamp, 0)
                .map(|dt| dt.format("%Y-%m-%d").to_string())
                .unwrap_or_else(|| "Unknown".to_string());
            println!("      {}. {} - {}", idx + 1, trade.side, trade.title.as_deref().unwrap_or("Unknown"));
            println!("         ${:.2} @ {}", trade.usdc_size, date);
        }
        println!();
    } else {
        println!("   ❌ No trades found on proxy wallet\n");
    }

    println!("{}\n", "━".repeat(65));

    // Check connection between addresses
    println!("🔗 CONNECTION BETWEEN ADDRESSES:\n");

    if let Some(first_trade) = eoa_activities.first() {
        println!("   EOA trades contain proxyWallet: {}", first_trade.proxy_wallet.as_deref().unwrap_or("N/A"));
    }

    if let Some(first_trade) = proxy_activities.first() {
        println!("   Proxy trades contain proxyWallet: {}", first_trade.proxy_wallet.as_deref().unwrap_or("N/A"));
    }

    println!("\n   💡 HOW IT WORKS:\n");
    println!("   1. EOA (Externally Owned Account) - your main wallet");
    println!("      • Controlled by private key");
    println!("      • Signs transactions");
    println!("      • Does NOT store funds on Polymarket\n");

    println!("   2. Proxy Wallet - smart contract wallet");
    println!("      • Created automatically by Polymarket");
    println!("      • Stores USDC and position tokens");
    println!("      • Executes trades on behalf of EOA");
    println!("      • Linked to EOA through signature\n");

    println!("{}\n", "━".repeat(65));

    // Identify the problem
    println!("❓ WHY NO STATISTICS ON PROFILE?\n");

    let eoa_has_trades = !eoa_activities.is_empty();
    let proxy_has_trades = !proxy_activities.is_empty();

    if !eoa_has_trades && proxy_has_trades {
        println!("   🎯 PROBLEM FOUND!\n");
        println!("   All trades go through Proxy Wallet, but statistics on Polymarket");
        println!("   may be displayed on the main wallet profile (EOA).\n");

        println!("   📊 WHERE TO VIEW STATISTICS:\n");
        println!("   ✅ CORRECT profile (with trading):");
        println!("      https://polymarket.com/profile/{}\n", env.proxy_wallet);

        println!("   ❌ EOA profile (may be empty):");
        println!("      https://polymarket.com/profile/{}\n", eoa_address);

        println!("   💡 SOLUTION:\n");
        println!("   Use Proxy Wallet address to view statistics:");
        println!("   {}\n", env.proxy_wallet);
    } else if eoa_has_trades && !proxy_has_trades {
        println!("   Trades go through main wallet (EOA)");
        println!("   Statistics should be displayed on EOA profile\n");
    } else if eoa_has_trades && proxy_has_trades {
        println!("   Trades exist on both addresses!");
        println!("   You may have used different wallets\n");
    } else {
        println!("   ❌ No trades found on any address\n");
    }

    println!("{}\n", "━".repeat(65));

    // Check via blockchain
    println!("🔗 BLOCKCHAIN CHECK:\n");
    println!("   EOA (main):");
    println!("   https://polygonscan.com/address/{}\n", eoa_address);
    println!("   Proxy Wallet (contract):");
    println!("   https://polygonscan.com/address/{}\n", env.proxy_wallet);

    // Check address type via RPC
    println!("   🔍 Address types:");
    let eoa_is_contract = is_contract_address(&env.rpc_url, &eoa_address).await.unwrap_or(false);
    let proxy_is_contract = is_contract_address(&env.rpc_url, &env.proxy_wallet).await.unwrap_or(false);
    
    println!("      EOA: {}", if !eoa_is_contract { "✅ Regular wallet (EOA)" } else { "⚠️  Smart contract" });
    println!("      Proxy: {}", if proxy_is_contract { "✅ Smart contract (correct)" } else { "❌ Regular wallet (error!)" });
    println!();

    println!("{}\n", "━".repeat(65));

    println!("✅ SUMMARY:\n");
    println!("   Your bot uses PROXY_WALLET for trading.");
    println!("   This is correct and safe!\n");
    println!("   Statistics and charts should be displayed at:");
    println!("   🔗 https://polymarket.com/profile/{}\n", env.proxy_wallet);
    println!("   If charts are still not there, this is a Polymarket UI bug.\n");

    Ok(())
}

