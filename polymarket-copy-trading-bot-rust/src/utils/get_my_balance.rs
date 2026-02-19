use anyhow::Result;
use alloy::primitives::U256;
use crate::config::Env;

pub async fn get_my_balance(address: &str, env: &Env) -> Result<f64> {
    // Using RPC directly to call balanceOf on USDC contract
    let client = reqwest::Client::new();
    
    // balanceOf(address) function selector: 0x70a08231
    // Pad address to 32 bytes (64 hex chars)
    let address_trimmed = address.trim_start_matches("0x");
    let padded_address = format!("{:0>64}", address_trimmed);
    let data = format!("0x70a08231{}", padded_address);
    
    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{
            "to": env.usdc_contract_address,
            "data": data
        }, "latest"],
        "id": 1
    });
    
    let response: serde_json::Value = client
        .post(&env.rpc_url)
        .json(&payload)
        .send()
        .await?
        .json()
        .await?;
    
    if let Some(result) = response.get("result").and_then(|r| r.as_str()) {
        let balance_hex = result.trim_start_matches("0x");
        let balance_u256 = U256::from_str_radix(balance_hex, 16)
            .map_err(|e| anyhow::anyhow!("Failed to parse balance: {}", e))?;
        // USDC has 6 decimals
        let balance_u64 = balance_u256.to::<u64>();
        let balance_f64 = balance_u64 as f64 / 1_000_000.0;
        return Ok(balance_f64);
    }
    
    anyhow::bail!("Failed to get balance from RPC response")
}

