//! Test auto-claim configuration & diagnostics
#![allow(dead_code)] // Struct fields used for JSON deserialization

use anyhow::Result;
use polymarket_copy_trading_bot_rust::config::load_env;
use polymarket_copy_trading_bot_rust::utils::fetch_data;

const RESOLVED_HIGH: f64 = 0.99;
const RESOLVED_LOW: f64 = 0.01;
const ZERO_THRESHOLD: f64 = 0.0001;

#[derive(Debug, serde::Deserialize)]
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

async fn get_balance(rpc_url: &str, address: &str) -> Result<f64> {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_getBalance",
        "params": [address, "latest"],
        "id": 1
    });
    let client = reqwest::Client::new();
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
    Ok(value_u64 as f64 / 1e18) // MATIC has 18 decimals
}

fn format_interval(ms: u64) -> String {
    let minutes = ms / 1000 / 60;
    let hours = minutes / 60;
    let days = hours / 24;
    
    if days >= 1 {
        format!("{:.1} days", days as f64)
    } else if hours >= 1 {
        format!("{:.1} hours", hours as f64)
    } else {
        format!("{} minutes", minutes)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("\n{}", "═".repeat(63));
    println!("🔍 AUTO-CLAIM CONFIGURATION TEST");
    println!("{}\n", "═".repeat(63));

    let env = load_env()?;

    // 1. Check Environment Variables
    println!("📋 STEP 1: Environment Configuration\n");
    println!("   AUTO_CLAIM_ENABLED: {}", if env.auto_claim_enabled { "✅ true" } else { "❌ false" });
    println!("   AUTO_CLAIM_INTERVAL_MS: {} ({})", env.auto_claim_interval_ms, format_interval(env.auto_claim_interval_ms));
    println!("   PROXY_WALLET: {}", env.proxy_wallet);
    println!("   RPC_URL: {}", env.rpc_url);
    println!("   USDC_CONTRACT: {}\n", env.usdc_contract_address);

    if !env.auto_claim_enabled {
        println!("⚠️  WARNING: Auto-claim is disabled in configuration\n");
    }

    // 2. Check Wallet Setup
    println!("{}\n", "━".repeat(65));
    println!("📋 STEP 2: Wallet Configuration\n");

    let signer_address = {
        use alloy::signers::local::PrivateKeySigner;
        
        use std::str::FromStr;
        let private_key = if env.private_key.starts_with("0x") {
            env.private_key.clone()
        } else {
            format!("0x{}", env.private_key)
        };
        let signer = PrivateKeySigner::from_str(&private_key)?;
        signer.address().to_string()
    };

    println!("   Signer Address (from PRIVATE_KEY): {}", signer_address);
    println!("   Proxy Wallet Address: {}\n", env.proxy_wallet);

    let proxy_is_contract = is_contract_address(&env.rpc_url, &env.proxy_wallet).await.unwrap_or(false);
    let signer_is_proxy = signer_address.to_lowercase() == env.proxy_wallet.to_lowercase();

    println!("   Proxy Wallet Type: {}", if proxy_is_contract { "🔷 Contract (Gnosis Safe?)" } else { "🔵 EOA (Regular Wallet)" });
    println!("   Signer = Proxy: {}\n", if signer_is_proxy { "✅ Yes" } else { "❌ No" });

    if proxy_is_contract && !signer_is_proxy {
        println!("   ⚠️  WARNING: Proxy wallet is a contract (likely Gnosis Safe)");
        println!("      Auto-claim cannot execute transactions automatically.");
        println!("      You will need to use manual redemption via Safe web interface.\n");
    }

    // Check MATIC balance
    let matic_balance = get_balance(&env.rpc_url, &signer_address).await.unwrap_or(0.0);
    println!("   MATIC Balance: {:.4} MATIC", matic_balance);
    
    if matic_balance < 0.01 {
        println!("   ⚠️  WARNING: Low MATIC balance! Recommended: > 0.1 MATIC\n");
    } else if matic_balance < 0.1 {
        println!("   ⚠️  CAUTION: MATIC balance is low. Recommended: > 0.1 MATIC\n");
    } else {
        println!("   ✅ MATIC balance is sufficient\n");
    }

    // 3. Check Positions
    println!("{}\n", "━".repeat(65));
    println!("📋 STEP 3: Position Analysis\n");

    let positions_url = format!("https://data-api.polymarket.com/positions?user={}", env.proxy_wallet);
    let positions_json: serde_json::Value = fetch_data(&positions_url, &env).await?;
    let all_positions: Vec<Position> = positions_json
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|v| serde_json::from_value(v).ok())
        .collect();

    let valid_positions: Vec<_> = all_positions.iter().filter(|pos| pos.size > ZERO_THRESHOLD).collect();

    println!("   Total Positions: {}\n", valid_positions.len());

    if valid_positions.is_empty() {
        println!("   ℹ️  No open positions found\n");
    } else {
        let resolved: Vec<_> = valid_positions.iter().filter(|pos| pos.cur_price >= RESOLVED_HIGH || pos.cur_price <= RESOLVED_LOW).collect();
        let unresolved: Vec<_> = valid_positions.iter().filter(|pos| pos.cur_price < RESOLVED_HIGH && pos.cur_price > RESOLVED_LOW).collect();
        let redeemable: Vec<_> = valid_positions.iter().filter(|pos| {
            (pos.cur_price >= RESOLVED_HIGH || pos.cur_price <= RESOLVED_LOW) && pos.redeemable == Some(true)
        }).collect();

        println!("   📊 Position Breakdown:");
        println!("      - Resolved: {}", resolved.len());
        println!("      - Unresolved: {}", unresolved.len());
        println!("      - Redeemable: {}\n", redeemable.len());

        if !redeemable.is_empty() {
            println!("   💰 Redeemable Positions:\n");
            for (index, pos) in redeemable.iter().enumerate() {
                let title = pos.title.as_deref().or(pos.slug.as_deref()).unwrap_or("Unknown");
                let status = if pos.cur_price >= RESOLVED_HIGH { "✅ Won" } else { "❌ Lost" };
                println!("      {}. {}", index + 1, title);
                println!("         Status: {} (Price: ${:.4})", status, pos.cur_price);
                println!("         Value: ${:.2}", pos.current_value);
                println!("         Size: {:.4}", pos.size);
                println!();
            }
        } else {
            println!("   ℹ️  No redeemable positions at this time\n");
            
            if !resolved.is_empty() {
                println!("   ⚠️  Note: Some positions are resolved but not yet redeemable.");
                println!("      They may need manual redemption or more time.\n");
            }
        }

        if !unresolved.is_empty() && unresolved.len() <= 5 {
            println!("   📈 Sample Unresolved Positions:\n");
            for (index, pos) in unresolved.iter().take(3).enumerate() {
                let title = pos.title.as_deref().or(pos.slug.as_deref()).unwrap_or("Unknown");
                println!("      {}. {}", index + 1, title);
                println!("         Current Price: ${:.4}", pos.cur_price);
                println!("         Value: ${:.2}\n", pos.current_value);
            }
        }
    }

    // 4. Summary
    println!("{}\n", "━".repeat(65));
    println!("📋 STEP 4: Summary & Recommendations\n");

    let mut issues = Vec::new();
    let mut recommendations = Vec::new();

    if !env.auto_claim_enabled {
        issues.push("Auto-claim is disabled");
        recommendations.push("Set AUTO_CLAIM_ENABLED=true in .env to enable");
    }

    if proxy_is_contract && !signer_is_proxy {
        issues.push("Proxy wallet is a contract (Gnosis Safe)");
        recommendations.push("Use manual redemption via Safe web interface: https://app.safe.global/");
        recommendations.push("Or transfer positions to EOA wallet for automatic redemption");
    }

    if matic_balance < 0.01 {
        issues.push("Insufficient MATIC for gas fees");
        recommendations.push("Add MATIC to your wallet (recommended: > 0.1 MATIC)");
    }

    if issues.is_empty() {
        println!("   ✅ Configuration looks good!\n");
        println!("   Next steps:");
        println!("      - Start the bot: cargo run --release");
        println!("      - Or test manually: cargo run --bin trigger_auto_claim\n");
    } else {
        println!("   ⚠️  Issues Found:\n");
        for (index, issue) in issues.iter().enumerate() {
            println!("      {}. {}", index + 1, issue);
        }
        println!();
        println!("   💡 Recommendations:\n");
        for (index, rec) in recommendations.iter().enumerate() {
            println!("      {}. {}", index + 1, rec);
        }
        println!();
    }

    println!("{}\n", "═".repeat(63));
    Ok(())
}

