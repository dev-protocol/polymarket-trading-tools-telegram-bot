//! Transfer USDC from proxy wallet to private key wallet
#![allow(dead_code)] // Struct fields used for JSON deserialization

use anyhow::Result;
use polymarket_copy_trading_bot_rust::config::load_env;
use alloy::signers::local::PrivateKeySigner;

use std::str::FromStr;

async fn is_contract_address(rpc_url: &str, address: &str) -> Result<bool> {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_getCode",
        "params": [address, "latest"],
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
    println!("💸 Transferring USDC from Proxy Wallet to Private Key Wallet");
    println!("════════════════════════════════════════════════════\n");

    let env = load_env()?;

    let private_key = if env.private_key.starts_with("0x") {
        env.private_key.clone()
    } else {
        format!("0x{}", env.private_key)
    };
    let signer = PrivateKeySigner::from_str(&private_key)?;
    let signer_address = signer.address().to_string();

    println!("📍 Proxy Wallet: {}", env.proxy_wallet);
    println!("📍 Signer Wallet (Private Key): {}\n", signer_address);

    // Check if proxy wallet is a contract
    let is_contract = is_contract_address(&env.rpc_url, &env.proxy_wallet).await?;
    let signer_is_proxy = signer_address.to_lowercase() == env.proxy_wallet.to_lowercase();

    if is_contract && !signer_is_proxy {
        println!("⚠️  WARNING: Proxy wallet is a contract (likely Gnosis Safe)");
        println!("   You cannot directly transfer from a contract wallet using this script.\n");
        println!("   📋 SOLUTION: Use Safe web interface\n");
        println!("   1. Go to https://app.safe.global/");
        println!("   2. Connect and find your Safe: {}", env.proxy_wallet);
        println!("   3. Go to \"Send\" tab");
        println!("   4. Select USDC token");
        println!("   5. Enter recipient: {}", signer_address);
        println!("   6. Enter amount and execute transaction\n");
        return Ok(());
    }

    if signer_is_proxy {
        println!("ℹ️  Signer wallet IS the proxy wallet - no transfer needed!");
        println!("   Your USDC is already in the wallet controlled by your private key.\n");
        return Ok(());
    }

    println!("💡 NOTE: Direct USDC transfer requires:");
    println!("   1. Building an ERC20 transfer transaction");
    println!("   2. Signing it with your private key");
    println!("   3. Sending it to the blockchain");
    println!("\n   For contract wallets (Gnosis Safe), use the Safe web interface.");
    println!("   For EOA wallets, ensure the signer has permission to transfer from proxy.\n");

    println!("📋 Current Configuration:");
    println!("   Proxy Wallet: {}", env.proxy_wallet);
    println!("   Signer Address: {}", signer_address);
    println!("   USDC Contract: {}", env.usdc_contract_address);
    println!("\n   To check balances, use: cargo run --bin check_my_stats\n");

    Ok(())
}

