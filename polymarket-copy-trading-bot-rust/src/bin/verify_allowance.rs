//! Verify USDC allowance status
#![allow(dead_code)] // Struct fields used for JSON deserialization

use anyhow::Result;
use polymarket_copy_trading_bot_rust::config::load_env;

async fn get_erc20_balance(rpc_url: &str, contract: &str, address: &str) -> Result<f64> {
    let addr_trimmed = address.trim().trim_start_matches("0x").to_lowercase();
    let addr_padded = format!("{:0>64}", addr_trimmed);
    let data = format!("0x70a08231{}", addr_padded);
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{"to": contract, "data": data}, "latest"],
        "id": 1
    });
    let resp = client.post(rpc_url).json(&body).send().await?;
    let json: serde_json::Value = resp.json().await?;
    let result = json.get("result").and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("No result in RPC response"))?;
    let hex = result.trim_start_matches("0x");
    if hex.is_empty() {
        return Ok(0.0);
    }
    let value = alloy::primitives::U256::from_str_radix(hex, 16)?;
    let value_u64 = value.to::<u64>();
    Ok(value_u64 as f64 / 1_000_000.0) // USDC has 6 decimals
}

async fn get_erc20_allowance(rpc_url: &str, contract: &str, owner: &str, spender: &str) -> Result<f64> {
    let o = owner.trim().trim_start_matches("0x").to_lowercase();
    let s = spender.trim().trim_start_matches("0x").to_lowercase();
    let data = format!("0xdd62ed3e{:0>64}{:0>64}", o, s);
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{"to": contract, "data": data}, "latest"],
        "id": 1
    });
    let resp = client.post(rpc_url).json(&body).send().await?;
    let json: serde_json::Value = resp.json().await?;
    let result = json.get("result").and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("No result in RPC response"))?;
    let hex = result.trim_start_matches("0x");
    if hex.is_empty() {
        return Ok(0.0);
    }
    let value = alloy::primitives::U256::from_str_radix(hex, 16)?;
    let value_u64 = value.to::<u64>();
    Ok(value_u64 as f64 / 1_000_000.0) // USDC has 6 decimals
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 Verifying USDC allowance status...\n");

    let env = load_env()?;
    // Polymarket Exchange contract address on Polygon
    let polymarket_exchange = "0x4bFb41d5B3570DeFd03C39a9A4D35d77Ee40f384";

    // Check balance
    let balance = get_erc20_balance(&env.rpc_url, &env.usdc_contract_address, &env.proxy_wallet).await?;
    
    // Check current allowance
    let allowance = get_erc20_allowance(
        &env.rpc_url,
        &env.usdc_contract_address,
        &env.proxy_wallet,
        polymarket_exchange,
    )
    .await?;

    println!("{}", "═".repeat(70));
    println!("📊 WALLET STATUS");
    println!("{}", "═".repeat(70));
    println!("💼 Wallet:     {}", env.proxy_wallet);
    println!("💵 USDC:       {:.6} USDC", balance);
    if allowance == 0.0 {
        println!("✅ Allowance:  0 USDC (NOT SET!)");
    } else {
        println!("✅ Allowance:  {:.6} USDC (SET!)", allowance);
    }
    println!("📍 Exchange:   {}", polymarket_exchange);
    println!("{}\n", "═".repeat(70));

    if allowance == 0.0 {
        println!("❌ PROBLEM: Allowance is NOT set!");
        println!("\n📝 TO FIX: Run the following command:");
        println!("   cargo run --bin check_allowance");
        println!("\nOR wait for your pending transaction to confirm:");
        println!("   https://polygonscan.com/address/{}", env.proxy_wallet);
        std::process::exit(1);
    } else if allowance < balance {
        println!("⚠️  WARNING: Allowance is less than your balance!");
        println!("   You may not be able to trade your full balance.");
        println!("\n   Balance:   {:.6} USDC", balance);
        println!("   Allowance: {:.6} USDC", allowance);
        println!("\n   Consider setting unlimited allowance:");
        println!("   cargo run --bin check_allowance");
        std::process::exit(1);
    } else {
        println!("✅ SUCCESS: Allowance is properly set!");
        println!("   You can start trading now.");
        println!("\n🚀 Start the bot:");
        println!("   cargo run --release");
        std::process::exit(0);
    }
}

