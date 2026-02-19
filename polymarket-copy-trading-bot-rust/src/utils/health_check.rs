use anyhow::Result;
use serde::{Deserialize, Serialize};
use crate::config::Env;
use crate::utils::{get_my_balance, fetch_data, logger::Logger};
use mongodb::Database;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    pub healthy: bool,
    pub checks: HealthChecks,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthChecks {
    pub database: CheckResult,
    pub rpc: CheckResult,
    pub balance: BalanceCheckResult,
    pub polymarket_api: CheckResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub status: String, // "ok" | "error"
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceCheckResult {
    pub status: String, // "ok" | "error" | "warning"
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub balance: Option<f64>,
}

pub async fn perform_health_check(db: &Database, env: &Env) -> Result<HealthCheckResult> {
    let mut checks = HealthChecks {
        database: CheckResult {
            status: "error".to_string(),
            message: "Not checked".to_string(),
        },
        rpc: CheckResult {
            status: "error".to_string(),
            message: "Not checked".to_string(),
        },
        balance: BalanceCheckResult {
            status: "error".to_string(),
            message: "Not checked".to_string(),
            balance: None,
        },
        polymarket_api: CheckResult {
            status: "error".to_string(),
            message: "Not checked".to_string(),
        },
    };

    // Check MongoDB connection
    match db.run_command(mongodb::bson::doc! { "ping": 1 }, None).await {
        Ok(_) => {
            checks.database = CheckResult {
                status: "ok".to_string(),
                message: "Connected".to_string(),
            };
        }
        Err(e) => {
            checks.database = CheckResult {
                status: "error".to_string(),
                message: format!("Connection failed: {}", e),
            };
        }
    }

    // Check RPC endpoint
    let client = reqwest::Client::new();
    let rpc_payload = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_blockNumber",
        "params": [],
        "id": 1
    });

    match tokio::time::timeout(
        std::time::Duration::from_secs(5),
        client.post(&env.rpc_url).json(&rpc_payload).send(),
    )
    .await
    {
        Ok(Ok(response)) => {
            if response.status().is_success() {
                if let Ok(data) = response.json::<serde_json::Value>().await {
                    if data.get("result").is_some() {
                        checks.rpc = CheckResult {
                            status: "ok".to_string(),
                            message: "RPC endpoint responding".to_string(),
                        };
                    } else {
                        checks.rpc = CheckResult {
                            status: "error".to_string(),
                            message: "Invalid RPC response".to_string(),
                        };
                    }
                } else {
                    checks.rpc = CheckResult {
                        status: "error".to_string(),
                        message: "Failed to parse RPC response".to_string(),
                    };
                }
            } else {
                checks.rpc = CheckResult {
                    status: "error".to_string(),
                    message: format!("HTTP {}", response.status()),
                };
            }
        }
        Ok(Err(e)) => {
            checks.rpc = CheckResult {
                status: "error".to_string(),
                message: format!("RPC check failed: {}", e),
            };
        }
        Err(_) => {
            checks.rpc = CheckResult {
                status: "error".to_string(),
                message: "RPC check timeout".to_string(),
            };
        }
    }

    // Check USDC balance
    match get_my_balance(&env.proxy_wallet, env).await {
        Ok(balance) => {
            if balance > 0.0 {
                if balance < 10.0 {
                    checks.balance = BalanceCheckResult {
                        status: "warning".to_string(),
                        message: format!("Low balance: ${:.2}", balance),
                        balance: Some(balance),
                    };
                } else {
                    checks.balance = BalanceCheckResult {
                        status: "ok".to_string(),
                        message: format!("Balance: ${:.2}", balance),
                        balance: Some(balance),
                    };
                }
            } else {
                checks.balance = BalanceCheckResult {
                    status: "error".to_string(),
                    message: "Zero balance".to_string(),
                    balance: None,
                };
            }
        }
        Err(e) => {
            checks.balance = BalanceCheckResult {
                status: "error".to_string(),
                message: format!("Balance check failed: {}", e),
                balance: None,
            };
        }
    }

    // Check Polymarket API
    let test_url = "https://data-api.polymarket.com/positions?user=0x0000000000000000000000000000000000000000";
    match fetch_data(test_url, env).await {
        Ok(_) => {
            checks.polymarket_api = CheckResult {
                status: "ok".to_string(),
                message: "API responding".to_string(),
            };
        }
        Err(e) => {
            checks.polymarket_api = CheckResult {
                status: "error".to_string(),
                message: format!("API check failed: {}", e),
            };
        }
    }

    // Determine overall health
    let healthy = checks.database.status == "ok"
        && checks.rpc.status == "ok"
        && checks.balance.status != "error"
        && checks.polymarket_api.status == "ok";

    Ok(HealthCheckResult {
        healthy,
        checks,
        timestamp: chrono::Utc::now().timestamp(),
    })
}

pub fn log_health_check(result: &HealthCheckResult) {
    Logger::separator();
    Logger::header("🏥 HEALTH CHECK");
    Logger::info(&format!(
        "Overall Status: {}",
        if result.healthy { "✅ Healthy" } else { "❌ Unhealthy" }
    ));
    Logger::info(&format!(
        "Database: {} {}",
        if result.checks.database.status == "ok" {
            "✅"
        } else {
            "❌"
        },
        result.checks.database.message
    ));
    Logger::info(&format!(
        "RPC: {} {}",
        if result.checks.rpc.status == "ok" { "✅" } else { "❌" },
        result.checks.rpc.message
    ));
    let balance_icon = match result.checks.balance.status.as_str() {
        "ok" => "✅",
        "warning" => "⚠️",
        _ => "❌",
    };
    Logger::info(&format!(
        "Balance: {} {}",
        balance_icon,
        result.checks.balance.message
    ));
    Logger::info(&format!(
        "Polymarket API: {} {}",
        if result.checks.polymarket_api.status == "ok" {
            "✅"
        } else {
            "❌"
        },
        result.checks.polymarket_api.message
    ));
    Logger::separator();
}

