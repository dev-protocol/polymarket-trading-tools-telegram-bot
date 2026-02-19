use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserActivity {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub proxy_wallet: String,
    pub timestamp: i64,
    pub condition_id: String,
    pub r#type: String,
    pub size: f64,
    pub usdc_size: f64,
    pub transaction_hash: String,
    pub price: f64,
    pub asset: String,
    pub side: String,
    pub outcome_index: i32,
    pub title: String,
    pub slug: String,
    pub icon: String,
    pub event_slug: String,
    pub outcome: String,
    pub name: String,
    pub pseudonym: String,
    pub bio: String,
    pub profile_image: String,
    pub profile_image_optimized: String,
    pub bot: bool,
    pub bot_executed_time: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub my_bought_size: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPosition {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub proxy_wallet: String,
    pub asset: String,
    pub condition_id: String,
    pub size: f64,
    pub avg_price: f64,
    pub initial_value: f64,
    pub current_value: f64,
    pub cash_pnl: f64,
    pub percent_pnl: f64,
    pub total_bought: f64,
    pub realized_pnl: f64,
    pub percent_realized_pnl: f64,
    pub cur_price: f64,
    pub redeemable: bool,
    pub mergeable: bool,
    pub title: String,
    pub slug: String,
    pub icon: String,
    pub event_slug: String,
    pub outcome: String,
    pub outcome_index: i32,
    pub opposite_outcome: String,
    pub opposite_asset: String,
    pub end_date: String,
    pub negative_risk: bool,
}

/// RTDS activity payload (from WebSocket) - keys from API are camelCase.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RtdsActivity {
    pub proxy_wallet: Option<String>,
    pub timestamp: Option<i64>,
    pub condition_id: Option<String>,
    #[serde(rename = "type")]
    pub activity_type: Option<String>,
    pub size: Option<f64>,
    pub price: Option<f64>,
    pub asset: Option<String>,
    pub side: Option<String>,
    pub outcome_index: Option<i32>,
    pub title: Option<String>,
    pub slug: Option<String>,
    pub icon: Option<String>,
    pub event_slug: Option<String>,
    pub outcome: Option<String>,
    pub name: Option<String>,
    pub transaction_hash: Option<String>,
}

impl RtdsActivity {
    pub fn usdc_size(&self) -> f64 {
        self.size.unwrap_or(0.0) * self.price.unwrap_or(0.0)
    }
}

