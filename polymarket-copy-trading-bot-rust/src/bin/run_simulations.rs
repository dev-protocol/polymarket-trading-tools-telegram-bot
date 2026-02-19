//! Run comprehensive simulations
#![allow(dead_code)] // Struct fields used for JSON deserialization

use anyhow::Result;
use colored::*;
use std::env;
use std::process::{Command, Stdio};

struct SimulationConfig {
    trader_address: String,
    history_days: i32,
    multiplier: f64,
    min_order_size: f64,
    max_trades: Option<usize>,
    tag: Option<String>,
}

const DEFAULT_TRADERS: &[&str] = &[
    "0x7c3db723f1d4d8cb9c550095203b686cb11e5c6b",
    "0x6bab41a0dc40d6dd4c1a915b8c01969479fd1292",
];

struct Preset {
    history_days: i32,
    max_trades: usize,
    multipliers: &'static [f64],
    tag: &'static str,
}

const QUICK_MULTIPLIERS: &[f64] = &[1.0, 2.0];
const STANDARD_MULTIPLIERS: &[f64] = &[0.5, 1.0, 2.0];
const FULL_MULTIPLIERS: &[f64] = &[0.5, 1.0, 2.0, 3.0];

const PRESETS: &[(&str, Preset)] = &[
    (
        "quick",
        Preset {
            history_days: 7,
            max_trades: 500,
            multipliers: QUICK_MULTIPLIERS,
            tag: "quick",
        },
    ),
    (
        "standard",
        Preset {
            history_days: 30,
            max_trades: 2000,
            multipliers: STANDARD_MULTIPLIERS,
            tag: "std",
        },
    ),
    (
        "full",
        Preset {
            history_days: 90,
            max_trades: 5000,
            multipliers: FULL_MULTIPLIERS,
            tag: "full",
        },
    ),
];

fn run_simulation(config: &SimulationConfig) -> Result<()> {
    println!("{}", "\n🚀 Starting simulation...".cyan());
    println!(
        "{}",
        format!("   Trader: {}...", &config.trader_address[..10.min(config.trader_address.len())])
            .bright_black()
    );
    println!(
        "{}",
        format!(
            "   Days: {}, Multiplier: {}x, MinOrder: ${:.2}",
            config.history_days, config.multiplier, config.min_order_size
        )
        .bright_black()
    );

    let mut cmd = Command::new("cargo");
    cmd.args(&["run", "--release", "--bin", "simulate_profitability"])
        .env("SIM_TRADER_ADDRESS", &config.trader_address)
        .env("SIM_HISTORY_DAYS", config.history_days.to_string())
        .env("SIM_MIN_ORDER_USD", config.min_order_size.to_string())
        .env("TRADE_MULTIPLIER", config.multiplier.to_string());

    if let Some(max_trades) = config.max_trades {
        cmd.env("SIM_MAX_TRADES", max_trades.to_string());
    }

    if let Some(ref tag) = config.tag {
        cmd.env("SIM_RESULT_TAG", tag);
    }

    cmd.stdout(Stdio::inherit()).stderr(Stdio::inherit());

    let status = cmd.status()?;
    if status.success() {
        println!("{}", "✓ Simulation completed\n".green());
        Ok(())
    } else {
        anyhow::bail!("Simulation failed with code {}", status.code().unwrap_or(-1));
    }
}

async fn run_batch(configs: &[SimulationConfig]) -> Result<()> {
    println!("\n{}", "═".repeat(80).cyan());
    println!("{}", "  📊 BATCH SIMULATION RUNNER".cyan());
    println!("{}\n", "═".repeat(80).cyan());

    println!("{}", format!("Total simulations to run: {}\n", configs.len()).yellow());

    for (i, config) in configs.iter().enumerate() {
        println!("{}", format!("\n[{}] Running simulation...", i + 1).bold());
        match run_simulation(config) {
            Ok(_) => {}
            Err(e) => {
                println!("{}", format!("Simulation {} failed, continuing with next...\n", i + 1).red());
                eprintln!("Error: {}", e);
            }
        }

        // Small delay between simulations
        if i < configs.len() - 1 {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    }

    println!("\n{}", "═".repeat(80).green());
    println!("{}", "  ✅ ALL SIMULATIONS COMPLETED".green());
    println!("{}\n", "═".repeat(80).green());
    Ok(())
}

fn generate_configs(preset_name: &str, traders: Option<&[String]>) -> Vec<SimulationConfig> {
    let preset = PRESETS
        .iter()
        .find(|(name, _)| *name == preset_name)
        .map(|(_, p)| p)
        .unwrap_or(&PRESETS[1].1); // Default to standard

    let trader_list = traders
        .filter(|t| !t.is_empty())
        .map(|t| t.to_vec())
        .unwrap_or_else(|| DEFAULT_TRADERS.iter().map(|s| s.to_string()).collect());

    let mut configs = Vec::new();

    for trader in trader_list {
        for &multiplier in preset.multipliers {
            let tag = format!("{}_{}m{}", preset.tag, multiplier, multiplier)
                .replace(".", "p");
            configs.push(SimulationConfig {
                trader_address: trader.clone(),
                history_days: preset.history_days,
                multiplier,
                min_order_size: 1.0,
                max_trades: Some(preset.max_trades),
                tag: Some(tag),
            });
        }
    }

    configs
}

fn print_help() {
    println!("{}", "\n📊 Simulation Runner - Usage\n".cyan());
    println!("Interactive mode:");
    println!("{}", "  cargo run --bin run_simulations\n".yellow());

    println!("Preset modes:");
    println!("{}", "  cargo run --bin run_simulations quick      # 7 days, 2 multipliers".yellow());
    println!("{}", "  cargo run --bin run_simulations standard   # 30 days, 3 multipliers (recommended)".yellow());
    println!("{}", "  cargo run --bin run_simulations full       # 90 days, 4 multipliers\n".yellow());

    println!("Custom mode:");
    println!("{}", "  cargo run --bin run_simulations custom <trader> [days] [multiplier]\n".yellow());

    println!("Examples:");
    println!("{}", "  cargo run --bin run_simulations custom 0x7c3d... 30 2.0".bright_black());
    println!("{}", "  cargo run --bin run_simulations standard\n".bright_black());
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() {
        // Interactive mode - use standard preset
        println!("{}", "\n🎮 Running standard preset simulations...\n".cyan());
        let configs = generate_configs("standard", None);
        run_batch(&configs).await?;
        return Ok(());
    }

    let command = &args[0];

    match command.as_str() {
        "quick" => {
            let configs = generate_configs("quick", None);
            run_batch(&configs).await?;
        }
        "standard" | "std" => {
            let configs = generate_configs("standard", None);
            run_batch(&configs).await?;
        }
        "full" => {
            let configs = generate_configs("full", None);
            run_batch(&configs).await?;
        }
        "custom" => {
            if args.len() < 2 {
                println!("{}", "Error: Trader address required for custom mode".red());
                println!("{}", "Usage: cargo run --bin run_simulations custom <trader_address> [days] [multiplier]".yellow());
                return Ok(());
            }

            let trader = args[1].clone();
            let days = args.get(2).and_then(|s| s.parse::<i32>().ok()).unwrap_or(30);
            let multiplier = args.get(3).and_then(|s| s.parse::<f64>().ok()).unwrap_or(1.0);

            let config = SimulationConfig {
                trader_address: trader.to_lowercase(),
                history_days: days,
                multiplier,
                min_order_size: 1.0,
                max_trades: None,
                tag: Some("custom".to_string()),
            };

            run_simulation(&config)?;
        }
        "help" | "--help" | "-h" => {
            print_help();
        }
        _ => {
            println!("{}", format!("Unknown command: {}\n", command).red());
            print_help();
        }
    }

    Ok(())
}
