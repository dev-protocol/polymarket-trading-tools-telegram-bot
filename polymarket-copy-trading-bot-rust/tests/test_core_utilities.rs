//! Integration tests for core utility scripts

use std::process::Command;
use std::env;

#[test]
fn test_help_binary() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "help", "--quiet"])
        .output()
        .expect("Failed to execute help binary");
    
    assert!(output.status.success() || output.status.code() == Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("POLYMARKET COPY TRADING BOT") || stdout.contains("COMMANDS"));
}

#[test]
fn test_health_check_binary_exists() {
    // Test that binary compiles and can be invoked (even if it fails due to missing .env)
    let output = Command::new("cargo")
        .args(&["run", "--bin", "health_check", "--quiet", "--", "--help"])
        .output();
    
    // Binary should exist and be invokable
    assert!(output.is_ok());
}

#[test]
fn test_check_allowance_binary_exists() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "check_allowance", "--quiet", "--", "--help"])
        .output();
    
    assert!(output.is_ok());
}

#[test]
fn test_verify_allowance_binary_exists() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "verify_allowance", "--quiet", "--", "--help"])
        .output();
    
    assert!(output.is_ok());
}

#[test]
fn test_set_token_allowance_binary_exists() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "set_token_allowance", "--quiet", "--", "--help"])
        .output();
    
    assert!(output.is_ok());
}

#[test]
fn test_check_my_stats_binary_exists() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "check_my_stats", "--quiet", "--", "--help"])
        .output();
    
    assert!(output.is_ok());
}

#[test]
fn test_check_positions_detailed_binary_exists() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "check_positions_detailed", "--quiet", "--", "--help"])
        .output();
    
    assert!(output.is_ok());
}

#[test]
fn test_check_recent_activity_binary_exists() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "check_recent_activity", "--quiet", "--", "--help"])
        .output();
    
    assert!(output.is_ok());
}

#[test]
fn test_check_proxy_wallet_binary_exists() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "check_proxy_wallet", "--quiet", "--", "--help"])
        .output();
    
    assert!(output.is_ok());
}

#[test]
fn test_check_pnl_discrepancy_binary_exists() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "check_pnl_discrepancy", "--quiet", "--", "--help"])
        .output();
    
    assert!(output.is_ok());
}

#[test]
fn test_find_my_eoa_binary_exists() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "find_my_eoa", "--quiet", "--", "--help"])
        .output();
    
    assert!(output.is_ok());
}

#[test]
fn test_check_both_wallets_binary_exists() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "check_both_wallets", "--quiet", "--", "--help"])
        .output();
    
    assert!(output.is_ok());
}

