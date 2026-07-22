use crate::services::screener::{
    CentralControllerRegistry, NormalizedDividend, NormalizedKline, ScreenerPeriod,
};
use chrono::NaiveDate;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControllerClassification {
    CentralSoe,
    LocalSoe,
    NonSoe,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScreenerUniverseRecord {
    pub code: String,
    pub exchange: String,
    pub name: String,
    pub eastmoney_secid: String,
    pub ordinary_equity: bool,
    pub market_cap_cny: f64,
    pub actual_controller: String,
    pub controller_classification: ControllerClassification,
}

#[derive(Debug, Clone)]
pub struct CentralControllerRegistrySnapshot {
    pub valuation_date: NaiveDate,
    pub registry: CentralControllerRegistry,
    pub source_url: String,
    pub source_date: String,
}

#[derive(Debug, Clone)]
pub struct RawDividend {
    pub ex_dividend_date: Option<NaiveDate>,
    pub cash_per_ten_shares: Option<f64>,
    pub distribution_kind: DividendKind,
    pub confirmed: bool,
    pub per_share_basis: PerShareBasis,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DividendKind {
    Cash,
    SpecialCash,
    NonCash,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerShareBasis {
    ExDate,
    Current,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct NormalizedQuote {
    pub code: String,
    pub exchange: String,
    pub price: f64,
    pub observed_at: String,
}

pub trait ScreenerSource {
    fn fetch_controller_registry(
        &self,
        valuation_date: NaiveDate,
    ) -> Result<CentralControllerRegistrySnapshot, String>;

    fn fetch_universe(&self, valuation_date: NaiveDate) -> Result<Vec<ScreenerUniverseRecord>, String>;

    fn fetch_dividends(
        &self,
        code: &str,
        exchange: &str,
        year: i32,
        valuation_date: NaiveDate,
    ) -> Result<Vec<NormalizedDividend>, String>;

    fn fetch_quote(&self, code: &str, exchange: &str) -> Result<NormalizedQuote, String>;

    fn fetch_klines(
        &self,
        code: &str,
        exchange: &str,
        period: ScreenerPeriod,
    ) -> Result<Vec<NormalizedKline>, String>;
}
