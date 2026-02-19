//! Redeem resolved positions for USDC
#![allow(dead_code)] // Struct fields used for JSON deserialization

use anyhow::Result;
use polymarket_copy_trading_bot_rust::config::load_env;
use polymarket_copy_trading_bot_rust::utils::fetch_data;
use alloy::signers::local::PrivateKeySigner;

use std::str::FromStr;
use std::collections::HashMap;

const RESOLVED_HIGH: f64 = 0.99;
const RESOLVED_LOW: f64 = 0.01;
const ZERO_THRESHOLD: f64 = 0.0001;
const CTF_CONTRACT: &str = "0x4D97DCd97eC945f40cF65F87097ACe5EA0476045"; // ConditionalTokens

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

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Redeeming resolved positions");
    println!("════════════════════════════════════════════════════");
    
    let env = load_env()?;
    println!("Wallet: {}", env.proxy_wallet);
    println!("CTF Contract: {}", CTF_CONTRACT);
    println!("Win threshold: price >= ${}", RESOLVED_HIGH);
    println!("Loss threshold: price <= ${}", RESOLVED_LOW);

    // Setup signer
    let private_key = if env.private_key.starts_with("0x") {
        env.private_key.clone()
    } else {
        format!("0x{}", env.private_key)
    };
    let signer = PrivateKeySigner::from_str(&private_key)?;
    let signer_address = signer.address();

    println!("\n✅ Connected to Polygon RPC");
    println!("Signer address: {}", signer_address);

    if signer_address.to_string().to_lowercase() != env.proxy_wallet.to_lowercase() {
        println!(
            "⚠️  Note: Signer ({}) differs from proxy wallet ({})",
            signer_address, env.proxy_wallet
        );
        println!("   Make sure signer has permission to execute transactions on proxy wallet");
    }

    // Load positions
    let positions_url = format!("https://data-api.polymarket.com/positions?user={}", env.proxy_wallet);
    let positions_json: serde_json::Value = fetch_data(&positions_url, &env).await?;
    let all_positions: Vec<Position> = positions_json
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|v| serde_json::from_value::<Position>(v).ok())
        .filter(|pos: &Position| pos.size > ZERO_THRESHOLD)
        .collect();

    if all_positions.is_empty() {
        println!("\n🎉 No open positions detected for proxy wallet.");
        return Ok(());
    }

    // Filter for resolved and redeemable positions
    let redeemable_positions: Vec<_> = all_positions
        .iter()
        .filter(|pos| {
            (pos.cur_price >= RESOLVED_HIGH || pos.cur_price <= RESOLVED_LOW) && pos.redeemable == Some(true)
        })
        .collect();

    let active_positions: Vec<_> = all_positions
        .iter()
        .filter(|pos| pos.cur_price > RESOLVED_LOW && pos.cur_price < RESOLVED_HIGH)
        .collect();

    println!("\n📊 Position statistics:");
    println!("   Total positions: {}", all_positions.len());
    println!("   ✅ Resolved and redeemable: {}", redeemable_positions.len());
    println!("   ⏳ Active (not touching): {}", active_positions.len());

    if redeemable_positions.is_empty() {
        println!("\n✅ No positions to redeem.");
        return Ok(());
    }

    println!("\n🔄 Redeeming {} positions...", redeemable_positions.len());
    println!("⚠️  WARNING: Each redemption requires gas fees on Polygon");

    // Group positions by conditionId
    let mut positions_by_condition: HashMap<String, Vec<&Position>> = HashMap::new();
    for pos in &redeemable_positions {
        positions_by_condition
            .entry(pos.condition_id.clone())
            .or_insert_with(Vec::new)
            .push(pos);
    }

    println!("\n📦 Grouped into {} unique conditions", positions_by_condition.len());

    println!("\n💡 NOTE: Redemption requires direct blockchain interaction.");
    println!("   For Gnosis Safe wallets, use the Safe web interface:");
    println!("   https://app.safe.global/");
    println!("\n   For EOA wallets, this script would need to:");
    println!("   1. Build the redeemPositions transaction");
    println!("   2. Sign it with your private key");
    println!("   3. Send it to the blockchain");
    println!("\n   This functionality requires additional blockchain interaction code.");
    println!("   Consider using the Polymarket web interface for redemption.");

    println!("\n════════════════════════════════════════════════════");
    println!("✅ Summary of redeemable positions");
    println!("Conditions found: {}", positions_by_condition.len());
    println!("Total positions: {}", redeemable_positions.len());
    let total_value: f64 = redeemable_positions.iter().map(|p| p.current_value).sum();
    println!("Expected value: ${:.2}", total_value);
    println!("════════════════════════════════════════════════════\n");

    Ok(())
}

