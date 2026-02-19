use anyhow::{Context, Result};
use regex::Regex;
use std::env;

use super::copy_strategy::{CopyStrategy, CopyStrategyConfig, parse_tiered_multipliers};

#[derive(Debug, Clone)]
#[allow(dead_code)] // Some fields kept for backward compatibility or future use
pub struct Env {
    pub user_addresses: Vec<String>,
    pub proxy_wallet: String,
    pub private_key: String,
    pub clob_http_url: String,
    pub clob_ws_url: String,
    pub fetch_interval: u64,
    pub too_old_timestamp: u64,
    pub retry_limit: u32,
    pub trade_multiplier: f64,
    pub copy_percentage: f64,
    pub copy_strategy_config: CopyStrategyConfig,
    pub request_timeout_ms: u64,
    pub network_retry_limit: u32,
    pub trade_aggregation_enabled: bool,
    pub trade_aggregation_window_seconds: u64,
    pub mongo_uri: String,
    pub rpc_url: String,
    pub auto_claim_enabled: bool,
    pub auto_claim_interval_ms: u64,
    pub db_cleanup_enabled: bool,
    pub usdc_contract_address: String,
    pub take_profit_percent: Option<f64>,
    pub stop_loss_percent: Option<f64>,
    pub tp_sl_check_interval_ms: u64,
    pub preview_mode: bool,
}

fn is_valid_ethereum_address(address: &str) -> bool {
    let re = Regex::new(r"^0x[a-fA-F0-9]{40}$").unwrap();
    re.is_match(address)
}

fn validate_required_env() -> Result<()> {
    let required = vec![
        "USER_ADDRESSES",
        "PROXY_WALLET",
        "PRIVATE_KEY",
        "CLOB_HTTP_URL",
        "CLOB_WS_URL",
        "MONGO_URI",
        "RPC_URL",
        "USDC_CONTRACT_ADDRESS",
    ];

    let mut missing = Vec::new();
    for key in &required {
        if env::var(key).is_err() {
            missing.push(*key);
        }
    }

    if !missing.is_empty() {
        eprintln!("\n❌ Configuration Error: Missing required environment variables\n");
        eprintln!("Missing variables: {}\n", missing.join(", "));
        eprintln!("🔧 Quick fix:");
        eprintln!("   1. Run the setup wizard: cargo run --bin setup");
        eprintln!("   2. Or manually create .env file with all required variables\n");
        eprintln!("📖 See docs/QUICK_START.md for detailed instructions\n");
        anyhow::bail!("Missing required environment variables: {}", missing.join(", "));
    }

    Ok(())
}

fn validate_addresses() -> Result<()> {
    if let Ok(proxy_wallet) = env::var("PROXY_WALLET") {
        if !is_valid_ethereum_address(&proxy_wallet) {
            eprintln!("\n❌ Invalid Wallet Address\n");
            eprintln!("Your PROXY_WALLET: {}", proxy_wallet);
            eprintln!("Expected format:    0x followed by 40 hexadecimal characters\n");
            eprintln!("Example: 0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0\n");
            anyhow::bail!("Invalid PROXY_WALLET address format: {}", proxy_wallet);
        }
    }

    if let Ok(usdc_address) = env::var("USDC_CONTRACT_ADDRESS") {
        if !is_valid_ethereum_address(&usdc_address) {
            eprintln!("\n❌ Invalid USDC Contract Address\n");
            eprintln!("Current value: {}", usdc_address);
            eprintln!("Default value: 0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174\n");
            anyhow::bail!("Invalid USDC_CONTRACT_ADDRESS format: {}", usdc_address);
        }
    }

    Ok(())
}

fn validate_numeric_config() -> Result<()> {
    let fetch_interval = env::var("FETCH_INTERVAL")
        .unwrap_or_else(|_| "1".to_string())
        .parse::<u64>()
        .context("Invalid FETCH_INTERVAL")?;
    if fetch_interval == 0 {
        anyhow::bail!("Invalid FETCH_INTERVAL: must be positive");
    }

    let retry_limit = env::var("RETRY_LIMIT")
        .unwrap_or_else(|_| "3".to_string())
        .parse::<u32>()
        .context("Invalid RETRY_LIMIT")?;
    if retry_limit < 1 || retry_limit > 10 {
        anyhow::bail!("Invalid RETRY_LIMIT: must be between 1 and 10");
    }

    let too_old_timestamp = env::var("TOO_OLD_TIMESTAMP")
        .unwrap_or_else(|_| "24".to_string())
        .parse::<u64>()
        .context("Invalid TOO_OLD_TIMESTAMP")?;
    if too_old_timestamp == 0 {
        anyhow::bail!("Invalid TOO_OLD_TIMESTAMP: must be positive");
    }

    let request_timeout = env::var("REQUEST_TIMEOUT_MS")
        .unwrap_or_else(|_| "10000".to_string())
        .parse::<u64>()
        .context("Invalid REQUEST_TIMEOUT_MS")?;
    if request_timeout < 1000 {
        anyhow::bail!("Invalid REQUEST_TIMEOUT_MS: must be at least 1000ms");
    }

    let network_retry_limit = env::var("NETWORK_RETRY_LIMIT")
        .unwrap_or_else(|_| "3".to_string())
        .parse::<u32>()
        .context("Invalid NETWORK_RETRY_LIMIT")?;
    if network_retry_limit < 1 || network_retry_limit > 10 {
        anyhow::bail!("Invalid NETWORK_RETRY_LIMIT: must be between 1 and 10");
    }

    Ok(())
}

fn validate_urls() -> Result<()> {
    if let Ok(clob_http_url) = env::var("CLOB_HTTP_URL") {
        if !clob_http_url.starts_with("http") {
            eprintln!("\n❌ Invalid CLOB_HTTP_URL\n");
            eprintln!("Current value: {}", clob_http_url);
            eprintln!("Default value: https://clob.polymarket.com/\n");
            anyhow::bail!("Invalid CLOB_HTTP_URL: must be a valid HTTP/HTTPS URL");
        }
    }

    if let Ok(clob_ws_url) = env::var("CLOB_WS_URL") {
        if !clob_ws_url.starts_with("ws") {
            eprintln!("\n❌ Invalid CLOB_WS_URL\n");
            eprintln!("Current value: {}", clob_ws_url);
            eprintln!("Default value: wss://ws-subscriptions-clob.polymarket.com/ws\n");
            anyhow::bail!("Invalid CLOB_WS_URL: must be a valid WebSocket URL");
        }
    }

    if let Ok(rpc_url) = env::var("RPC_URL") {
        if !rpc_url.starts_with("http") {
            eprintln!("\n❌ Invalid RPC_URL\n");
            eprintln!("Current value: {}", rpc_url);
            eprintln!("Must start with: http:// or https://\n");
            anyhow::bail!("Invalid RPC_URL: must be a valid HTTP/HTTPS URL");
        }
    }

    if let Ok(mongo_uri) = env::var("MONGO_URI") {
        if !mongo_uri.starts_with("mongodb") {
            eprintln!("\n❌ Invalid MONGO_URI\n");
            eprintln!("Current value: {}", mongo_uri);
            eprintln!("Must start with: mongodb:// or mongodb+srv://\n");
            anyhow::bail!("Invalid MONGO_URI: must be a valid MongoDB connection string");
        }
    }

    Ok(())
}

fn parse_user_addresses(input: &str) -> Result<Vec<String>> {
    let trimmed = input.trim();
    
    // Check if it's JSON array format
    if trimmed.starts_with('[') && trimmed.ends_with(']') {
        let parsed: Vec<String> = serde_json::from_str(trimmed)
            .context("Invalid JSON format for USER_ADDRESSES")?;
        
        let addresses: Vec<String> = parsed
            .into_iter()
            .map(|addr| addr.to_lowercase().trim().to_string())
            .filter(|addr| !addr.is_empty())
            .collect();
        
        // Validate each address
        for addr in &addresses {
            if !is_valid_ethereum_address(addr) {
                eprintln!("\n❌ Invalid Trader Address in USER_ADDRESSES\n");
                eprintln!("Invalid address: {}", addr);
                eprintln!("Expected format: 0x followed by 40 hexadecimal characters\n");
                anyhow::bail!("Invalid Ethereum address in USER_ADDRESSES: {}", addr);
            }
        }
        
        return Ok(addresses);
    }
    
    // Otherwise treat as comma-separated
    let addresses: Vec<String> = trimmed
        .split(',')
        .map(|addr| addr.to_lowercase().trim().to_string())
        .filter(|addr| !addr.is_empty())
        .collect();
    
    // Validate each address
    for addr in &addresses {
        if !is_valid_ethereum_address(addr) {
            eprintln!("\n❌ Invalid Trader Address in USER_ADDRESSES\n");
            eprintln!("Invalid address: {}", addr);
            eprintln!("Expected format: 0x followed by 40 hexadecimal characters\n");
            anyhow::bail!("Invalid Ethereum address in USER_ADDRESSES: {}", addr);
        }
    }
    
    Ok(addresses)
}

fn parse_copy_strategy() -> Result<CopyStrategyConfig> {
    // Support legacy COPY_PERCENTAGE + TRADE_MULTIPLIER for backward compatibility
    let has_legacy_config = env::var("COPY_PERCENTAGE").is_ok() && env::var("COPY_STRATEGY").is_err();

    if has_legacy_config {
        eprintln!("⚠️  Using legacy COPY_PERCENTAGE configuration. Consider migrating to COPY_STRATEGY.");
        let copy_percentage = env::var("COPY_PERCENTAGE")
            .unwrap_or_else(|_| "10.0".to_string())
            .parse::<f64>()
            .context("Invalid COPY_PERCENTAGE")?;
        let trade_multiplier = env::var("TRADE_MULTIPLIER")
            .unwrap_or_else(|_| "1.0".to_string())
            .parse::<f64>()
            .context("Invalid TRADE_MULTIPLIER")?;
        let effective_percentage = copy_percentage * trade_multiplier;

        let mut config = CopyStrategyConfig {
            strategy: CopyStrategy::Percentage,
            copy_size: effective_percentage,
            max_order_size_usd: env::var("MAX_ORDER_SIZE_USD")
                .unwrap_or_else(|_| "100.0".to_string())
                .parse::<f64>()
                .context("Invalid MAX_ORDER_SIZE_USD")?,
            min_order_size_usd: env::var("MIN_ORDER_SIZE_USD")
                .unwrap_or_else(|_| "0.01".to_string())
                .parse::<f64>()
                .context("Invalid MIN_ORDER_SIZE_USD")?,
            max_position_size_usd: env::var("MAX_POSITION_SIZE_USD")
                .ok()
                .and_then(|v| v.parse::<f64>().ok()),
            max_daily_volume_usd: env::var("MAX_DAILY_VOLUME_USD")
                .ok()
                .and_then(|v| v.parse::<f64>().ok()),
            ..Default::default()
        };

        // Parse tiered multipliers if configured
        if let Ok(tiered_multipliers_str) = env::var("TIERED_MULTIPLIERS") {
            config.tiered_multipliers = Some(parse_tiered_multipliers(&tiered_multipliers_str)?);
            println!("✓ Loaded {} tiered multipliers", config.tiered_multipliers.as_ref().unwrap().len());
        } else if trade_multiplier != 1.0 {
            config.trade_multiplier = Some(trade_multiplier);
        }

        return Ok(config);
    }

    // Parse new copy strategy configuration
    let strategy_str = env::var("COPY_STRATEGY")
        .unwrap_or_else(|_| "PERCENTAGE".to_string())
        .to_uppercase();
    let strategy = match strategy_str.as_str() {
        "PERCENTAGE" => CopyStrategy::Percentage,
        "FIXED" => CopyStrategy::Fixed,
        "ADAPTIVE" => CopyStrategy::Adaptive,
        _ => CopyStrategy::Percentage,
    };

    let mut config = CopyStrategyConfig {
        strategy,
        copy_size: env::var("COPY_SIZE")
            .unwrap_or_else(|_| "10.0".to_string())
            .parse::<f64>()
            .context("Invalid COPY_SIZE")?,
        max_order_size_usd: env::var("MAX_ORDER_SIZE_USD")
            .unwrap_or_else(|_| "100.0".to_string())
            .parse::<f64>()
            .context("Invalid MAX_ORDER_SIZE_USD")?,
        min_order_size_usd: env::var("MIN_ORDER_SIZE_USD")
            .unwrap_or_else(|_| "0.01".to_string())
            .parse::<f64>()
            .context("Invalid MIN_ORDER_SIZE_USD")?,
        max_position_size_usd: env::var("MAX_POSITION_SIZE_USD")
            .ok()
            .and_then(|v| v.parse::<f64>().ok()),
        max_daily_volume_usd: env::var("MAX_DAILY_VOLUME_USD")
            .ok()
            .and_then(|v| v.parse::<f64>().ok()),
        ..Default::default()
    };

    // Add adaptive strategy parameters if applicable
    if strategy == CopyStrategy::Adaptive {
        config.adaptive_min_percent = Some(
            env::var("ADAPTIVE_MIN_PERCENT")
                .unwrap_or_else(|_| config.copy_size.to_string())
                .parse::<f64>()
                .context("Invalid ADAPTIVE_MIN_PERCENT")?,
        );
        config.adaptive_max_percent = Some(
            env::var("ADAPTIVE_MAX_PERCENT")
                .unwrap_or_else(|_| config.copy_size.to_string())
                .parse::<f64>()
                .context("Invalid ADAPTIVE_MAX_PERCENT")?,
        );
        config.adaptive_threshold = Some(
            env::var("ADAPTIVE_THRESHOLD_USD")
                .unwrap_or_else(|_| "500.0".to_string())
                .parse::<f64>()
                .context("Invalid ADAPTIVE_THRESHOLD_USD")?,
        );
    }

    // Parse tiered multipliers if configured
    if let Ok(tiered_multipliers_str) = env::var("TIERED_MULTIPLIERS") {
        config.tiered_multipliers = Some(parse_tiered_multipliers(&tiered_multipliers_str)?);
        println!("✓ Loaded {} tiered multipliers", config.tiered_multipliers.as_ref().unwrap().len());
    } else if let Ok(trade_multiplier_str) = env::var("TRADE_MULTIPLIER") {
        let single_multiplier = trade_multiplier_str.parse::<f64>().context("Invalid TRADE_MULTIPLIER")?;
        if single_multiplier != 1.0 {
            config.trade_multiplier = Some(single_multiplier);
            println!("✓ Using single trade multiplier: {}x", single_multiplier);
        }
    }

    Ok(config)
}

pub fn load_env() -> Result<Env> {
    dotenvy::dotenv().ok(); // Load .env file if it exists

    // Run all validations
    validate_required_env()?;
    validate_addresses()?;
    validate_numeric_config()?;
    validate_urls()?;

    let user_addresses_str = env::var("USER_ADDRESSES")
        .context("USER_ADDRESSES is required")?;
    let user_addresses = parse_user_addresses(&user_addresses_str)?;

    Ok(Env {
        user_addresses,
        proxy_wallet: env::var("PROXY_WALLET").context("PROXY_WALLET is required")?,
        private_key: env::var("PRIVATE_KEY").context("PRIVATE_KEY is required")?,
        clob_http_url: env::var("CLOB_HTTP_URL").context("CLOB_HTTP_URL is required")?,
        clob_ws_url: env::var("CLOB_WS_URL").context("CLOB_WS_URL is required")?,
        fetch_interval: env::var("FETCH_INTERVAL")
            .unwrap_or_else(|_| "1".to_string())
            .parse::<u64>()
            .unwrap_or(1),
        too_old_timestamp: env::var("TOO_OLD_TIMESTAMP")
            .unwrap_or_else(|_| "24".to_string())
            .parse::<u64>()
            .unwrap_or(24),
        retry_limit: env::var("RETRY_LIMIT")
            .unwrap_or_else(|_| "3".to_string())
            .parse::<u32>()
            .unwrap_or(3),
        trade_multiplier: env::var("TRADE_MULTIPLIER")
            .unwrap_or_else(|_| "1.0".to_string())
            .parse::<f64>()
            .unwrap_or(1.0),
        copy_percentage: env::var("COPY_PERCENTAGE")
            .unwrap_or_else(|_| "10.0".to_string())
            .parse::<f64>()
            .unwrap_or(10.0),
        copy_strategy_config: parse_copy_strategy()?,
        request_timeout_ms: env::var("REQUEST_TIMEOUT_MS")
            .unwrap_or_else(|_| "10000".to_string())
            .parse::<u64>()
            .unwrap_or(10000),
        network_retry_limit: env::var("NETWORK_RETRY_LIMIT")
            .unwrap_or_else(|_| "3".to_string())
            .parse::<u32>()
            .unwrap_or(3),
        trade_aggregation_enabled: env::var("TRADE_AGGREGATION_ENABLED")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .unwrap_or(false),
        trade_aggregation_window_seconds: env::var("TRADE_AGGREGATION_WINDOW_SECONDS")
            .unwrap_or_else(|_| "300".to_string())
            .parse::<u64>()
            .unwrap_or(300),
        mongo_uri: env::var("MONGO_URI").context("MONGO_URI is required")?,
        rpc_url: env::var("RPC_URL").context("RPC_URL is required")?,
        auto_claim_enabled: env::var("AUTO_CLAIM_ENABLED")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .unwrap_or(false),
        auto_claim_interval_ms: env::var("AUTO_CLAIM_INTERVAL_MS")
            .unwrap_or_else(|_| "3600000".to_string())
            .parse::<u64>()
            .unwrap_or(3600000),
        db_cleanup_enabled: env::var("DB_CLEANUP_ENABLED")
            .unwrap_or_else(|_| "true".to_string())
            .parse::<bool>()
            .unwrap_or(true),
        usdc_contract_address: env::var("USDC_CONTRACT_ADDRESS")
            .context("USDC_CONTRACT_ADDRESS is required")?,
        take_profit_percent: env::var("TAKE_PROFIT_PERCENT")
            .ok()
            .and_then(|v| v.parse::<f64>().ok())
            .filter(|&v| v > 0.0),
        stop_loss_percent: env::var("STOP_LOSS_PERCENT")
            .ok()
            .and_then(|v| v.parse::<f64>().ok())
            .filter(|&v| v > 0.0),
        tp_sl_check_interval_ms: env::var("TP_SL_CHECK_INTERVAL_MS")
            .unwrap_or_else(|_| "1000".to_string())
            .parse::<u64>()
            .unwrap_or(1000),
        preview_mode: env::var("PREVIEW_MODE")
            .unwrap_or_else(|_| "true".to_string())
            .parse::<bool>()
            .unwrap_or(true),
    })
}

