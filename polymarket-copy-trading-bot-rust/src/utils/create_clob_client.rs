use anyhow::Result;
use polymarket_client_sdk::clob::Client as ClobClient;
use polymarket_client_sdk::clob::types::SignatureType;
use polymarket_client_sdk::auth::state::Authenticated;
use polymarket_client_sdk::auth::Normal;
use polymarket_client_sdk::POLYGON;
use alloy::signers::local::PrivateKeySigner;
use alloy::signers::Signer as _;
use std::str::FromStr;
use crate::config::Env;
use crate::utils::logger::Logger;

async fn is_contract_address(rpc_url: &str, address: &str) -> Result<bool> {
    let addr_trimmed = address.trim().trim_start_matches("0x");
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_getCode",
        "params": [format!("0x{}", addr_trimmed), "latest"],
        "id": 1
    });
    let client = reqwest::Client::new();
    let resp = client
        .post(rpc_url)
        .json(&body)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await?;
    let json: serde_json::Value = resp.json().await?;
    let result = json
        .get("result")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("No result in RPC response"))?;
    // If code is not "0x" or empty, it's a contract
    let code = result.trim_start_matches("0x");
    Ok(!code.is_empty() && code.chars().any(|c| c != '0'))
}

pub async fn create_clob_client(env: &Env) -> Result<(ClobClient<Authenticated<Normal>>, PrivateKeySigner)> {
    let chain_id = POLYGON;
    let host = &env.clob_http_url;
    
    // Create signer from private key
    let private_key = if env.private_key.starts_with("0x") {
        env.private_key.clone()
    } else {
        format!("0x{}", env.private_key)
    };
    let signer = PrivateKeySigner::from_str(&private_key)
        .map_err(|e| anyhow::anyhow!("Invalid private key: {}", e))?
        .with_chain_id(Some(chain_id));
    
    // Detect if the proxy wallet is a Gnosis Safe or EOA
    let is_proxy_safe = is_contract_address(&env.rpc_url, &env.proxy_wallet).await?;
    
    let wallet_type = if is_proxy_safe {
        "Gnosis Safe"
    } else {
        "EOA (Externally Owned Account)"
    };
    let signature_type = if is_proxy_safe {
        SignatureType::GnosisSafe
    } else {
        SignatureType::Eoa
    };
    
    Logger::info(&format!("Wallet type detected: {}", wallet_type));
    
    // Create CLOB client and authenticate
    let clob_client = ClobClient::new(host, Default::default())?;
    let authenticated = clob_client
        .authentication_builder(&signer)
        .signature_type(signature_type)
        .authenticate()
        .await?;
    
    Ok((authenticated, signer))
}

