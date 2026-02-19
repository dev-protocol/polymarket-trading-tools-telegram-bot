use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CopyStrategy {
    #[default]
    #[serde(rename = "PERCENTAGE")]
    Percentage,
    #[serde(rename = "FIXED")]
    Fixed,
    #[serde(rename = "ADAPTIVE")]
    Adaptive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiplierTier {
    pub min: f64,
    pub max: Option<f64>, // None = infinity
    pub multiplier: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CopyStrategyConfig {
    pub strategy: CopyStrategy,
    pub copy_size: f64,
    pub adaptive_min_percent: Option<f64>,
    pub adaptive_max_percent: Option<f64>,
    pub adaptive_threshold: Option<f64>,
    pub tiered_multipliers: Option<Vec<MultiplierTier>>,
    pub trade_multiplier: Option<f64>,
    pub max_order_size_usd: f64,
    pub min_order_size_usd: f64,
    pub max_position_size_usd: Option<f64>,
    pub max_daily_volume_usd: Option<f64>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields may be used for debugging/logging
pub struct OrderSizeCalculation {
    pub trader_order_size: f64,
    pub base_amount: f64,
    pub final_amount: f64,
    pub strategy: CopyStrategy,
    pub capped_by_max: bool,
    pub reduced_by_balance: bool,
    pub below_minimum: bool,
    pub reasoning: String,
}

pub fn calculate_order_size(
    config: &CopyStrategyConfig,
    trader_order_size: f64,
    available_balance: f64,
    current_position_size: f64,
) -> OrderSizeCalculation {
    let (base_amount, mut reasoning) = match config.strategy {
        CopyStrategy::Percentage => {
            let amount = trader_order_size * (config.copy_size / 100.0);
            let reason = format!(
                "{}% of trader's ${:.2} = ${:.2}",
                config.copy_size, trader_order_size, amount
            );
            (amount, reason)
        }
        CopyStrategy::Fixed => {
            let amount = config.copy_size;
            let reason = format!("Fixed amount: ${:.2}", amount);
            (amount, reason)
        }
        CopyStrategy::Adaptive => {
            let adaptive_percent = calculate_adaptive_percent(config, trader_order_size);
            let amount = trader_order_size * (adaptive_percent / 100.0);
            let reason = format!(
                "Adaptive {:.1}% of trader's ${:.2} = ${:.2}",
                adaptive_percent, trader_order_size, amount
            );
            (amount, reason)
        }
    };

    // Apply tiered or single multiplier
    let multiplier = get_trade_multiplier(config, trader_order_size);
    let mut final_amount = base_amount * multiplier;

    if multiplier != 1.0 {
        reasoning.push_str(&format!(
            " → {}x multiplier: ${:.2} → ${:.2}",
            multiplier, base_amount, final_amount
        ));
    }

    let mut capped_by_max = false;
    let mut reduced_by_balance = false;
    let mut below_minimum = false;

    // Apply maximum order size limit
    if final_amount > config.max_order_size_usd {
        final_amount = config.max_order_size_usd;
        capped_by_max = true;
        reasoning.push_str(&format!(" → Capped at max ${}", config.max_order_size_usd));
    }

    // Apply maximum position size limit
    if let Some(max_position_size) = config.max_position_size_usd {
        let new_total_position = current_position_size + final_amount;
        if new_total_position > max_position_size {
            let allowed_amount = (max_position_size - current_position_size).max(0.0);
            if allowed_amount < config.min_order_size_usd {
                final_amount = 0.0;
                reasoning.push_str(" → Position limit reached");
            } else {
                final_amount = allowed_amount;
                reasoning.push_str(" → Reduced to fit position limit");
            }
        }
    }

    // Check available balance (with 1% safety buffer)
    let max_affordable = available_balance * 0.99;
    if final_amount > max_affordable {
        final_amount = max_affordable;
        reduced_by_balance = true;
        reasoning.push_str(&format!(" → Reduced to fit balance (${:.2})", max_affordable));
    }

    // Check minimum order size
    if final_amount < config.min_order_size_usd {
        below_minimum = true;
        reasoning.push_str(&format!(" → Below minimum ${}", config.min_order_size_usd));
        final_amount = config.min_order_size_usd;
    }

    OrderSizeCalculation {
        trader_order_size,
        base_amount,
        final_amount,
        strategy: config.strategy,
        capped_by_max,
        reduced_by_balance,
        below_minimum,
        reasoning,
    }
}

fn calculate_adaptive_percent(config: &CopyStrategyConfig, trader_order_size: f64) -> f64 {
    let min_percent = config.adaptive_min_percent.unwrap_or(config.copy_size);
    let max_percent = config.adaptive_max_percent.unwrap_or(config.copy_size);
    let threshold = config.adaptive_threshold.unwrap_or(500.0);

    if trader_order_size >= threshold {
        // Large order: scale down to minPercent
        let factor = (trader_order_size / threshold - 1.0).min(1.0);
        lerp(config.copy_size, min_percent, factor)
    } else {
        // Small order: scale up to maxPercent
        let factor = trader_order_size / threshold;
        lerp(max_percent, config.copy_size, factor)
    }
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    let clamped_t = t.max(0.0).min(1.0);
    a + (b - a) * clamped_t
}

#[allow(dead_code)] // May be used for validation in future
pub fn validate_copy_strategy_config(config: &CopyStrategyConfig) -> Vec<String> {
    let mut errors = Vec::new();

    // Validate copySize
    if config.copy_size <= 0.0 {
        errors.push("copySize must be positive".to_string());
    }

    if config.strategy == CopyStrategy::Percentage && config.copy_size > 100.0 {
        errors.push("copySize for PERCENTAGE strategy should be <= 100".to_string());
    }

    // Validate limits
    if config.max_order_size_usd <= 0.0 {
        errors.push("maxOrderSizeUSD must be positive".to_string());
    }

    if config.min_order_size_usd <= 0.0 {
        errors.push("minOrderSizeUSD must be positive".to_string());
    }

    if config.min_order_size_usd > config.max_order_size_usd {
        errors.push("minOrderSizeUSD cannot be greater than maxOrderSizeUSD".to_string());
    }

    // Validate adaptive parameters
    if config.strategy == CopyStrategy::Adaptive {
        if config.adaptive_min_percent.is_none() || config.adaptive_max_percent.is_none() {
            errors.push("ADAPTIVE strategy requires adaptiveMinPercent and adaptiveMaxPercent".to_string());
        }

        if let (Some(min), Some(max)) = (config.adaptive_min_percent, config.adaptive_max_percent) {
            if min > max {
                errors.push("adaptiveMinPercent cannot be greater than adaptiveMaxPercent".to_string());
            }
        }
    }

    errors
}

#[allow(dead_code)] // May be used for setup wizard
pub fn get_recommended_config(balance_usd: f64) -> CopyStrategyConfig {
    if balance_usd < 500.0 {
        // Small balance: Conservative
        CopyStrategyConfig {
            strategy: CopyStrategy::Percentage,
            copy_size: 5.0,
            max_order_size_usd: 20.0,
            min_order_size_usd: 1.0,
            max_position_size_usd: Some(50.0),
            max_daily_volume_usd: Some(100.0),
            ..Default::default()
        }
    } else if balance_usd < 2000.0 {
        // Medium balance: Balanced
        CopyStrategyConfig {
            strategy: CopyStrategy::Percentage,
            copy_size: 10.0,
            max_order_size_usd: 50.0,
            min_order_size_usd: 1.0,
            max_position_size_usd: Some(200.0),
            max_daily_volume_usd: Some(500.0),
            ..Default::default()
        }
    } else {
        // Large balance: Adaptive
        CopyStrategyConfig {
            strategy: CopyStrategy::Adaptive,
            copy_size: 10.0,
            adaptive_min_percent: Some(5.0),
            adaptive_max_percent: Some(15.0),
            adaptive_threshold: Some(300.0),
            max_order_size_usd: 100.0,
            min_order_size_usd: 1.0,
            max_position_size_usd: Some(1000.0),
            max_daily_volume_usd: Some(2000.0),
            ..Default::default()
        }
    }
}

pub fn parse_tiered_multipliers(tiers_str: &str) -> Result<Vec<MultiplierTier>> {
    if tiers_str.trim().is_empty() {
        return Ok(Vec::new());
    }

    let mut tiers = Vec::new();
    let tier_defs: Vec<&str> = tiers_str
        .split(',')
        .map(|t| t.trim())
        .filter(|t| !t.is_empty())
        .collect();

    for tier_def in tier_defs {
        let parts: Vec<&str> = tier_def.split(':').collect();
        if parts.len() != 2 {
            anyhow::bail!(
                "Invalid tier format: \"{}\". Expected \"min-max:multiplier\" or \"min+:multiplier\"",
                tier_def
            );
        }

        let range = parts[0].trim();
        let multiplier = parts[1]
            .trim()
            .parse::<f64>()
            .context(format!("Invalid multiplier in tier \"{}\"", tier_def))?;

        if multiplier < 0.0 {
            anyhow::bail!("Invalid multiplier in tier \"{}\": must be >= 0", tier_def);
        }

        // Parse range
        if range.ends_with('+') {
            // Infinite upper bound: "500+"
            let min = range[..range.len() - 1]
                .parse::<f64>()
                .context(format!("Invalid minimum value in tier \"{}\"", tier_def))?;
            if min < 0.0 {
                anyhow::bail!("Invalid minimum value in tier \"{}\": must be >= 0", tier_def);
            }
            tiers.push(MultiplierTier {
                min,
                max: None,
                multiplier,
            });
        } else if let Some(dash_pos) = range.find('-') {
            // Bounded range: "100-500"
            let min_str = &range[..dash_pos];
            let max_str = &range[dash_pos + 1..];
            let min = min_str
                .parse::<f64>()
                .context(format!("Invalid minimum value in tier \"{}\"", tier_def))?;
            let max = max_str
                .parse::<f64>()
                .context(format!("Invalid maximum value in tier \"{}\"", tier_def))?;

            if min < 0.0 {
                anyhow::bail!("Invalid minimum value in tier \"{}\": must be >= 0", tier_def);
            }
            if max <= min {
                anyhow::bail!(
                    "Invalid maximum value in tier \"{}\": must be > {}",
                    tier_def,
                    min
                );
            }

            tiers.push(MultiplierTier {
                min,
                max: Some(max),
                multiplier,
            });
        } else {
            anyhow::bail!(
                "Invalid range format in tier \"{}\". Use \"min-max\" or \"min+\"",
                tier_def
            );
        }
    }

    // Sort tiers by min value
    tiers.sort_by(|a, b| a.min.partial_cmp(&b.min).unwrap_or(std::cmp::Ordering::Equal));

    // Validate no overlaps and no gaps
    for i in 0..tiers.len() - 1 {
        let current = &tiers[i];
        let next = &tiers[i + 1];

        if current.max.is_none() {
            anyhow::bail!(
                "Tier with infinite upper bound must be last: {}+",
                current.min
            );
        }

        if let Some(current_max) = current.max {
            if current_max > next.min {
                anyhow::bail!(
                    "Overlapping tiers: [{}-{}] and [{}-{}]",
                    current.min,
                    current_max,
                    next.min,
                    next.max.map_or("∞".to_string(), |m| m.to_string())
                );
            }
        }
    }

    Ok(tiers)
}

pub fn get_trade_multiplier(config: &CopyStrategyConfig, trader_order_size: f64) -> f64 {
    // Use tiered multipliers if configured
    if let Some(ref tiered_multipliers) = config.tiered_multipliers {
        if !tiered_multipliers.is_empty() {
            for tier in tiered_multipliers {
                if trader_order_size >= tier.min {
                    if tier.max.is_none() || trader_order_size < tier.max.unwrap() {
                        return tier.multiplier;
                    }
                }
            }
            // If no tier matches, use the last tier's multiplier
            if let Some(last_tier) = tiered_multipliers.last() {
                return last_tier.multiplier;
            }
        }
    }

    // Fall back to single multiplier if configured
    if let Some(multiplier) = config.trade_multiplier {
        return multiplier;
    }

    // Default: no multiplier
    1.0
}

impl fmt::Display for CopyStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CopyStrategy::Percentage => write!(f, "PERCENTAGE"),
            CopyStrategy::Fixed => write!(f, "FIXED"),
            CopyStrategy::Adaptive => write!(f, "ADAPTIVE"),
        }
    }
}

