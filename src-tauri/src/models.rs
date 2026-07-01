use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Account {
    pub id: i64,
    pub name: String,
    pub r#type: String,
    pub balance: f64,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub r#type: String,
    pub icon: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Transaction {
    pub id: i64,
    pub r#type: String,
    pub amount: f64,
    pub category_id: Option<i64>,
    pub account_id: Option<i64>,
    pub note: String,
    pub transaction_date: String,
    pub created_at: String,
    pub category_name: Option<String>,
    pub category_icon: Option<String>,
    pub account_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NewTransaction {
    pub r#type: String,
    pub amount: f64,
    pub category_id: Option<i64>,
    pub account_id: Option<i64>,
    pub note: Option<String>,
    pub transaction_date: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Holding {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub r#type: String,
    pub quantity: f64,
    pub cost_price: f64,
    pub current_price: f64,
    pub market: String,
    pub created_at: String,
    pub updated_at: String,
    pub market_value: f64,
    pub cost_value: f64,
    pub profit: f64,
    pub profit_rate: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NewHolding {
    pub code: String,
    pub name: String,
    pub r#type: String,
    pub quantity: f64,
    pub cost_price: f64,
    pub market: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PricePoint {
    pub price: f64,
    pub recorded_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DashboardSummary {
    pub total_assets: f64,
    pub account_balance: f64,
    pub holding_value: f64,
    pub holding_profit: f64,
    pub month_income: f64,
    pub month_expense: f64,
    pub month_balance: f64,
    pub holding_count: i64,
    pub transaction_count: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MonthlyStat {
    pub month: String,
    pub income: f64,
    pub expense: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AssetDistribution {
    pub name: String,
    pub value: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub quote_update_interval: i64,
    pub quote_update_enabled: bool,
    pub currency: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QuoteRefreshResult {
    pub updated: usize,
    pub failed: usize,
    pub message: String,
}
