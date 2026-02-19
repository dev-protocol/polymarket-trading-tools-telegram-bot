//! Find EOA address from private key
#![allow(dead_code)] // Struct fields used for JSON deserialization

use anyhow::Result;
use alloy::signers::local::PrivateKeySigner;

use std::str::FromStr;
use polymarket_copy_trading_bot_rust::config::load_env;
use polymarket_copy_trading_bot_rust::utils::fetch_data;

#[tokio::main]
async fn main() -> Result<()> {
    println!("\n🔍 WALLET AND ADDRESS ANALYSIS\n");
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

    println!("📋 STEP 1: Address from private key (EOA)\n");
    println!("   {}\n", eoa_address);

    // Show PROXY_WALLET from .env
    println!("📋 STEP 2: PROXY_WALLET from .env\n");
    println!("   {}\n", env.proxy_wallet);

    // Compare
    println!("{}\n", "━".repeat(65));
    println!("🔎 COMPARISON:\n");

    if eoa_address.to_lowercase() == env.proxy_wallet.to_lowercase() {
        println!("   ⚠️  EOA AND PROXY_WALLET ARE THE SAME ADDRESS!\n");
        println!("   This means .env has EOA address, not proxy wallet.\n");
        println!("   Polymarket should have created a separate proxy wallet for this EOA,");
        println!("   but the bot uses the EOA directly.\n");
    } else {
        println!("   ✅ EOA and PROXY_WALLET are different addresses\n");
        println!("   EOA (owner):        {}", eoa_address);
        println!("   PROXY (for trading): {}\n", env.proxy_wallet);
    }

    // Check if PROXY_WALLET is a smart contract
    println!("{}\n", "━".repeat(65));
    println!("📋 STEP 3: Check PROXY_WALLET type\n");

    let code_body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_getCode",
        "params": [env.proxy_wallet, "latest"],
        "id": 1
    });
    let client = reqwest::Client::new();
    let code_resp = client.post(&env.rpc_url).json(&code_body).send().await?;
    let code_json: serde_json::Value = code_resp.json().await?;
    let code = code_json.get("result").and_then(|v| v.as_str()).unwrap_or("0x");
    let is_contract = code != "0x" && code.chars().any(|c| c != '0');

    if is_contract {
        println!("   ✅ PROXY_WALLET is a smart contract (Gnosis Safe)\n");
        println!("   This is the correct configuration for Polymarket.\n");
    } else {
        println!("   ⚠️  PROXY_WALLET is NOT a smart contract!\n");
        println!("   This is a regular EOA address.\n");
        println!("   Polymarket usually uses a Gnosis Safe proxy.\n");
    }

    // Check activity on both addresses
    println!("{}\n", "━".repeat(65));
    println!("📋 STEP 4: Activity on Polymarket\n");

    let proxy_positions_url = format!("https://data-api.polymarket.com/positions?user={}", env.proxy_wallet);
    let proxy_positions_json: serde_json::Value = fetch_data(&proxy_positions_url, &env).await?;
    let proxy_positions: Vec<serde_json::Value> = proxy_positions_json
        .as_array()
        .cloned()
        .unwrap_or_default();
    
    println!("   PROXY_WALLET ({}...):", &env.proxy_wallet[..10.min(env.proxy_wallet.len())]);
    println!("   • Positions: {}\n", proxy_positions.len());

    if eoa_address.to_lowercase() != env.proxy_wallet.to_lowercase() {
        let eoa_positions_url = format!("https://data-api.polymarket.com/positions?user={}", eoa_address);
        let eoa_positions_json: serde_json::Value = fetch_data(&eoa_positions_url, &env).await?;
        let eoa_positions: Vec<serde_json::Value> = eoa_positions_json
            .as_array()
            .cloned()
            .unwrap_or_default();
        
        println!("   EOA ({}...):", &eoa_address[..10.min(eoa_address.len())]);
        println!("   • Positions: {}\n", eoa_positions.len());
    }

    // Check connection via activity API
    println!("{}\n", "━".repeat(65));
    println!("📋 STEP 5: Check proxyWallet in transactions\n");

    let activities_url = format!("https://data-api.polymarket.com/activity?user={}&type=TRADE", env.proxy_wallet);
    let activities_json: serde_json::Value = fetch_data(&activities_url, &env).await?;
    let activities: Vec<serde_json::Value> = activities_json
        .as_array()
        .cloned()
        .unwrap_or_default();

    if let Some(first_trade) = activities.first() {
        let proxy_wallet_in_trade = first_trade.get("proxyWallet")
            .and_then(|v| v.as_str())
            .unwrap_or("N/A");

        println!("   Address from .env:         {}", env.proxy_wallet);
        println!("   proxyWallet in trades:     {}\n", proxy_wallet_in_trade);

        if proxy_wallet_in_trade.to_lowercase() == env.proxy_wallet.to_lowercase() {
            println!("   ✅ Addresses match!\n");
        } else {
            println!("   ⚠️  ADDRESSES DO NOT MATCH!\n");
            println!("   This may mean Polymarket uses a different proxy.\n");
        }
    }

    // Instructions
    println!("{}\n", "━".repeat(65));
    println!("💡 HOW TO ACCESS POSITIONS ON FRONTEND:\n");
    println!("{}\n", "━".repeat(65));

    println!("🔧 OPTION 1: Import private key into MetaMask\n");
    println!("   1. Open MetaMask");
    println!("   2. Click account icon -> Import Account");
    println!("   3. Paste your PRIVATE_KEY from .env file");
    println!("   4. Connect to Polymarket with this account");
    println!("   5. Polymarket will automatically show the correct proxy wallet\n");

    println!("⚠️  WARNING: Never share your private key!\n");

    println!("{}\n", "━".repeat(65));
    println!("🔧 OPTION 2: Find proxy wallet via URL\n");
    println!("   Your positions are available at:\n");
    println!("   https://polymarket.com/profile/{}\n", env.proxy_wallet);
    println!("   Open this link in browser to view.\n");

    println!("{}\n", "━".repeat(65));
    println!("🔧 OPTION 3: Check via Polygon Explorer\n");
    println!("   https://polygonscan.com/address/{}\n", env.proxy_wallet);
    println!("   Here you can see all transactions and tokens.\n");

    println!("{}\n", "━".repeat(65));

    // Additional information
    println!("📚 ADDITIONAL INFORMATION:\n");
    println!("   • EOA (Externally Owned Account) - your main wallet");
    println!("   • Proxy Wallet - smart contract for trading on Polymarket");
    println!("   • One EOA can have only one proxy wallet on Polymarket");
    println!("   • All positions are stored in proxy wallet, not in EOA\n");

    println!("{}\n", "━".repeat(65));

    // Export connection information
    println!("📋 CONNECTION DATA:\n");
    println!("   EOA address:       {}", eoa_address);
    println!("   Proxy address:    {}", env.proxy_wallet);
    println!("   Proxy type:       {}\n", if is_contract { "Smart Contract (Gnosis Safe)" } else { "EOA (simple address)" });

    println!("{}\n", "━".repeat(65));

    Ok(())
}

