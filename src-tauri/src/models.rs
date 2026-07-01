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
pub struct NewAccount {
    pub name: String,
    pub r#type: String,
    pub balance: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UpdateAccount {
    pub name: String,
    pub r#type: String,
    pub balance: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub r#type: String,
    pub icon: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NewCategory {
    pub name: String,
    pub r#type: String,
    pub icon: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UpdateCategory {
    pub name: String,
    pub icon: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Transaction {
    pub id: i64,
    pub r#type: String,
    pub amount: f64,
    pub category_id: Option<i64>,
    pub account_id: Option<i64>,
    pub transfer_to_account_id: Option<i64>,
    pub note: String,
    pub transaction_date: String,
    pub created_at: String,
    pub category_name: Option<String>,
    pub category_icon: Option<String>,
    pub account_name: Option<String>,
    pub transfer_to_account_name: Option<String>,
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
pub struct UpdateTransaction {
    pub r#type: String,
    pub amount: f64,
    pub category_id: Option<i64>,
    pub account_id: Option<i64>,
    pub note: Option<String>,
    pub transaction_date: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TransferInput {
    pub from_account_id: i64,
    pub to_account_id: i64,
    pub amount: f64,
    pub note: Option<String>,
    pub transaction_date: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct TransactionFilter {
    pub month: Option<String>,
    pub account_id: Option<i64>,
    pub category_id: Option<i64>,
    pub tx_type: Option<String>,
    pub keyword: Option<String>,
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
    pub liquid_assets: f64,
    pub broker_balance: f64,
    pub account_balance: f64,
    pub holding_value: f64,
    pub holding_profit: f64,
    pub month_income: f64,
    pub month_expense: f64,
    pub month_balance: f64,
    pub holding_count: i64,
    pub transaction_count: i64,
    pub is_empty: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MonthlyStat {
    pub month: String,
    pub income: f64,
    pub expense: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CategoryStat {
    pub category_id: i64,
    pub category_name: String,
    pub category_icon: String,
    pub amount: f64,
    pub percentage: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub quote_update_interval: i64,
    pub quote_update_enabled: bool,
    pub refresh_on_startup: bool,
    #[serde(default = "default_currency")]
    pub currency: String,
    #[serde(default)]
    pub ai_enabled: bool,
    #[serde(default)]
    pub ai_api_key: String,
    #[serde(default = "default_ai_api_base")]
    pub ai_api_base: String,
    #[serde(default = "default_ai_model")]
    pub ai_model: String,
    #[serde(default = "default_theme_mode")]
    pub theme_mode: String,
}

fn default_currency() -> String {
    "CNY".to_string()
}

fn default_ai_api_base() -> String {
    "https://v2.pincc.ai/v1".to_string()
}

fn default_ai_model() -> String {
    "gpt-4o-mini".to_string()
}

fn default_theme_mode() -> String {
    "system".to_string()
}

#[derive(Debug, Clone)]
pub struct AiConfig {
    pub ai_enabled: bool,
    pub ai_api_key: String,
    pub ai_api_base: String,
    pub ai_model: String,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            ai_enabled: true,
            ai_api_key: String::new(),
            ai_api_base: "https://v2.pincc.ai/v1".to_string(),
            ai_model: "gpt-4o-mini".to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ParsedTransactionDraft {
    pub r#type: String,
    pub amount: Option<f64>,
    pub category_id: Option<i64>,
    pub category_name: Option<String>,
    pub account_id: Option<i64>,
    pub account_name: Option<String>,
    pub transaction_date: String,
    pub note: String,
    pub confidence: f64,
    pub source: String,
    pub raw_text: String,
    pub parse_notice: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ParsedTransactionBatch {
    pub raw_text: String,
    pub items: Vec<ParsedTransactionDraft>,
    pub parse_notice: Option<String>,
    pub source: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QuoteRefreshResult {
    pub updated: usize,
    pub failed: usize,
    pub message: String,
    pub failed_codes: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KlineBar {
    pub date: String,
    pub open: f64,
    pub close: f64,
    pub low: f64,
    pub high: f64,
    pub volume: f64,
    pub change_pct: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KlineData {
    pub bars: Vec<KlineBar>,
    pub chart_type: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExportPayload {
    pub version: String,
    pub exported_at: String,
    pub accounts: Vec<Account>,
    pub categories: Vec<Category>,
    pub transactions: Vec<TransactionRow>,
    pub holdings: Vec<HoldingRow>,
    pub settings: Vec<(String, String)>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TransactionRow {
    pub r#type: String,
    pub amount: f64,
    pub category_id: Option<i64>,
    pub account_id: Option<i64>,
    pub transfer_to_account_id: Option<i64>,
    pub note: String,
    pub transaction_date: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HoldingRow {
    pub code: String,
    pub name: String,
    pub r#type: String,
    pub quantity: f64,
    pub cost_price: f64,
    pub current_price: f64,
    pub market: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Budget {
    pub id: i64,
    pub category_id: i64,
    pub category_name: String,
    pub category_icon: String,
    pub month: String,
    pub amount: f64,
    pub spent: f64,
    pub remaining: f64,
    pub usage_rate: f64,
    pub is_over: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NewBudget {
    pub category_id: i64,
    pub month: String,
    pub amount: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BudgetAlert {
    pub category_name: String,
    pub category_icon: String,
    pub budget: f64,
    pub spent: f64,
    pub over_amount: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RecurringRule {
    pub id: i64,
    pub r#type: String,
    pub amount: f64,
    pub category_id: Option<i64>,
    pub account_id: Option<i64>,
    pub note: String,
    pub day_of_month: i64,
    pub enabled: bool,
    pub last_run_month: Option<String>,
    pub category_name: Option<String>,
    pub category_icon: Option<String>,
    pub account_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NewRecurringRule {
    pub r#type: String,
    pub amount: f64,
    pub category_id: Option<i64>,
    pub account_id: Option<i64>,
    pub note: Option<String>,
    pub day_of_month: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PortfolioHistoryPoint {
    pub date: String,
    pub total_value: f64,
}
