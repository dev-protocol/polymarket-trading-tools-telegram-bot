//! Common test utilities and helpers

use std::env;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Create a temporary .env file for testing
pub fn create_test_env_file() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let env_path = temp_dir.path().join(".env");
    
    let test_env_content = r#"# Test Environment Variables
USER_ADDRESSES=0x1111111111111111111111111111111111111111,0x2222222222222222222222222222222222222222
PROXY_WALLET=0x3333333333333333333333333333333333333333
PRIVATE_KEY=0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef
CLOB_HTTP_URL=https://clob.polymarket.com
CLOB_WS_URL=wss://ws-subscriptions-clob.polymarket.com/ws
MONGO_URI=mongodb://localhost:27017/test
RPC_URL=https://polygon-rpc.com
USDC_CONTRACT_ADDRESS=0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174
FETCH_INTERVAL=1
RETRY_LIMIT=3
TRADE_MULTIPLIER=1.0
COPY_PERCENTAGE=10.0
COPY_STRATEGY=PERCENTAGE
MAX_ORDER_SIZE_USD=100.0
MIN_ORDER_SIZE_USD=0.01
AUTO_CLAIM_ENABLED=false
TAKE_PROFIT_PERCENT=10.0
STOP_LOSS_PERCENT=10.0
"#;
    
    fs::write(&env_path, test_env_content).expect("Failed to write test .env file");
    
    // Set environment variable to point to temp directory
    env::set_var("TEST_ENV_DIR", temp_dir.path().to_str().unwrap());
    
    temp_dir
}

/// Setup test environment
pub fn setup_test_env() -> TempDir {
    let temp_dir = create_test_env_file();
    
    // Change to temp directory for tests
    let original_dir = env::current_dir().unwrap();
    env::set_var("ORIGINAL_DIR", original_dir.to_str().unwrap());
    
    temp_dir
}

/// Cleanup test environment
pub fn cleanup_test_env(temp_dir: TempDir) {
    temp_dir.close().expect("Failed to cleanup temp directory");
}

/// Check if we're in a test environment
pub fn is_test_mode() -> bool {
    env::var("TEST_MODE").is_ok()
}

/// Mock HTTP responses for testing
pub mod mocks {
    use serde_json::json;
    
    pub fn mock_positions_response() -> serde_json::Value {
        json!([])
    }
    
    pub fn mock_activities_response() -> serde_json::Value {
        json!([])
    }
    
    pub fn mock_leaderboard_response() -> serde_json::Value {
        json!([
            {
                "address": "0x1111111111111111111111111111111111111111",
                "pnl": 1000.0,
                "win_rate": 0.65,
                "total_trades": 150,
                "volume": 50000.0
            }
        ])
    }
}

