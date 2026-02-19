//! Check and set USDC allowance for Polymarket trading
#![allow(dead_code)] // Struct fields used for JSON deserialization

use anyhow::Result;

use polymarket_copy_trading_bot_rust::config::load_env;

// USDC ABI functions we need
const BALANCE_OF_SELECTOR: &str = "0x70a08231"; // balanceOf(address)
const ALLOWANCE_SELECTOR: &str = "0xdd62ed3e"; // allowance(address,address)
#[allow(dead_code)]
const APPROVE_SELECTOR: &str = "0x095ea7b3"; // approve(address,uint256)
const DECIMALS_SELECTOR: &str = "0x313ce567"; // decimals()

async fn get_erc20_decimals(rpc_url: &str, contract: &str) -> Result<u8> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{"to": contract, "data": DECIMALS_SELECTOR}, "latest"],
        "id": 1
    });
    let resp = client.post(rpc_url).json(&body).send().await?;
    let json: serde_json::Value = resp.json().await?;
    let result = json.get("result").and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("No result in RPC response"))?;
    let hex = result.trim_start_matches("0x");
    if hex.is_empty() {
        return Ok(6); // Default to 6 for USDC
    }
    let value = u8::from_str_radix(&hex[hex.len().saturating_sub(2)..], 16).unwrap_or(6);
    Ok(value)
}

async fn get_erc20_balance(rpc_url: &str, contract: &str, address: &str) -> Result<f64> {
    let addr_trimmed = address.trim().trim_start_matches("0x").to_lowercase();
    let addr_padded = format!("{:0>64}", addr_trimmed);
    let data = format!("{}{}", BALANCE_OF_SELECTOR, addr_padded);
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
    let decimals = get_erc20_decimals(rpc_url, contract).await.unwrap_or(6);
    
    // Check if it's MaxUint256 (all F's) - effectively infinite
    let trimmed_hex = hex.trim_start_matches('0');
    let is_max = trimmed_hex.len() == 64 && trimmed_hex.chars().all(|c| c == 'f' || c == 'F');
    if is_max {
        // MaxUint256 - effectively infinite
        return Ok(f64::INFINITY);
    }
    
    // Parse the hex value - use u128 to handle large numbers
    let value = if hex.len() <= 32 {
        u128::from_str_radix(hex, 16).unwrap_or(0) as f64
    } else {
        // Very large number - parse in chunks
        let last_32 = &hex[hex.len().saturating_sub(32)..];
        let base = u128::from_str_radix(last_32, 16).unwrap_or(0) as f64;
        let higher_order_hex = &hex[..hex.len().saturating_sub(32)];
        if !higher_order_hex.is_empty() {
            let multiplier = 16_f64.powi(higher_order_hex.len() as i32);
            base + (u128::from_str_radix(higher_order_hex, 16).unwrap_or(0) as f64 * multiplier)
        } else {
            base
        }
    };
    
    Ok(value / 10_f64.powi(decimals as i32))
}

async fn get_erc20_allowance(rpc_url: &str, contract: &str, owner: &str, spender: &str) -> Result<f64> {
    let o = owner.trim().trim_start_matches("0x").to_lowercase();
    let s = spender.trim().trim_start_matches("0x").to_lowercase();
    let data = format!("{}{:0>64}{:0>64}", ALLOWANCE_SELECTOR, o, s);
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
    let decimals = get_erc20_decimals(rpc_url, contract).await.unwrap_or(6);
    
    // Check if it's MaxUint256 (all F's) - effectively infinite
    let trimmed_hex = hex.trim_start_matches('0');
    let is_max = trimmed_hex.len() == 64 && trimmed_hex.chars().all(|c| c == 'f' || c == 'F');
    if is_max {
        // MaxUint256 - effectively infinite allowance
        return Ok(f64::INFINITY);
    }
    
    // Parse the hex value - use u128 to handle large numbers
    let value = if hex.len() <= 32 {
        u128::from_str_radix(hex, 16).unwrap_or(0) as f64
    } else {
        // Very large number - parse in chunks
        let last_32 = &hex[hex.len().saturating_sub(32)..];
        let base = u128::from_str_radix(last_32, 16).unwrap_or(0) as f64;
        let higher_order_hex = &hex[..hex.len().saturating_sub(32)];
        if !higher_order_hex.is_empty() {
            let multiplier = 16_f64.powi(higher_order_hex.len() as i32);
            base + (u128::from_str_radix(higher_order_hex, 16).unwrap_or(0) as f64 * multiplier)
        } else {
            base
        }
    };
    
    Ok(value / 10_f64.powi(decimals as i32))
}

#[allow(dead_code)]
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
    println!("🔍 Checking USDC balance and allowance...\n");

    let env = load_env()?;
    let decimals = get_erc20_decimals(&env.rpc_url, &env.usdc_contract_address).await?;
    println!("💵 USDC Decimals: {}\n", decimals);

    // Polymarket Exchange contract address on Polygon
    let polymarket_exchange = "0x4bFb41d5B3570DeFd03C39a9A4D8dE6Bd8B8982E";
    let _polymarket_collateral = "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174"; // USDC.e

    // Get balance and allowance
    let balance = get_erc20_balance(&env.rpc_url, &env.usdc_contract_address, &env.proxy_wallet).await?;
    let allowance = get_erc20_allowance(&env.rpc_url, &env.usdc_contract_address, &env.proxy_wallet, polymarket_exchange).await?;

    println!("💰 Your USDC Balance ({}): {:.6} USDC", env.usdc_contract_address, balance);
    if allowance.is_infinite() {
        println!("✅ Current Allowance: ∞ (unlimited)");
    } else {
        println!("✅ Current Allowance: {:.6} USDC", allowance);
    }
    println!("📍 Polymarket Exchange: {}\n", polymarket_exchange);

    if allowance.is_infinite() || (allowance >= balance && allowance > 0.0) {
        println!("✅ Allowance is already sufficient! No action needed.");
    } else {
        println!("⚠️  Allowance is insufficient or zero!");
        println!("📝 To set unlimited allowance for Polymarket:\n");
        println!("💡 Use a wallet interface (MetaMask, etc.) to call:");
        println!("   Contract: {}", env.usdc_contract_address);
        println!("   Function: approve({}, MAX_UINT256)", polymarket_exchange);
        println!("\n   Or implement full transaction signing in this script.");
        println!("   The allowance check shows you need to approve: {:.6} USDC", balance);
    }

    Ok(())
}

