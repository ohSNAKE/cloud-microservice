use crate::services::screener::{
    CentralControllerRegistry, NormalizedDividend, NormalizedKline, ScreenerPeriod,
};
use crate::services::screener_source_eastmoney_universe::{
    classify_controller, collect_exchange_pages, validate_complete_universe, EastmoneyUniversePage,
    EastmoneyUniverseRow,
};
use crate::services::screener_source_sasac::{parse_sasac_directory, SASAC_DIRECTORY_URL};
use chrono::NaiveDate;
use std::collections::HashMap;

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

#[derive(Clone)]
pub struct EastmoneyScreenerSource<EastmoneyGetter, SasacGetter> {
    eastmoney_getter: EastmoneyGetter,
    sasac_getter: SasacGetter,
}

impl<EastmoneyGetter, SasacGetter> EastmoneyScreenerSource<EastmoneyGetter, SasacGetter>
where
    EastmoneyGetter: Fn(&str) -> Result<String, String> + Clone,
    SasacGetter: Fn(&str) -> Result<String, String> + Clone,
{
    pub fn with_getters(eastmoney_getter: EastmoneyGetter, sasac_getter: SasacGetter) -> Self {
        Self {
            eastmoney_getter,
            sasac_getter,
        }
    }

    fn fetch_exchange_universe(
        &self,
        exchange: &str,
        registry: &CentralControllerRegistry,
    ) -> Result<Vec<ScreenerUniverseRecord>, String> {
        let payload = (self.eastmoney_getter)(&format!("universe:{exchange}"))?;
        let mut controllers = HashMap::new();
        let rows = payload
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| parse_universe_line(exchange, line, &mut controllers))
            .collect::<Result<Vec<_>, _>>()?;
        let mut records = collect_exchange_pages(
            exchange,
            vec![EastmoneyUniversePage {
                total_pages: 1,
                page_no: 1,
                rows,
            }],
        )?;
        for record in &mut records {
            let actual_controller = controllers
                .remove(&(record.code.clone(), record.exchange.clone()))
                .unwrap_or_default();
            record.controller_classification = classify_controller(&actual_controller, registry);
            record.actual_controller = actual_controller;
        }
        Ok(records)
    }
}

impl<EastmoneyGetter, SasacGetter> ScreenerSource
    for EastmoneyScreenerSource<EastmoneyGetter, SasacGetter>
where
    EastmoneyGetter: Fn(&str) -> Result<String, String> + Clone,
    SasacGetter: Fn(&str) -> Result<String, String> + Clone,
{
    fn fetch_controller_registry(
        &self,
        valuation_date: NaiveDate,
    ) -> Result<CentralControllerRegistrySnapshot, String> {
        let html = (self.sasac_getter)(SASAC_DIRECTORY_URL)?;
        let snapshot = parse_sasac_directory(&valuation_date.to_string(), &html)?;
        Ok(CentralControllerRegistrySnapshot {
            valuation_date,
            registry: CentralControllerRegistry::from_entries(snapshot.entries),
            source_url: snapshot.source_url,
            source_date: snapshot.source_date,
        })
    }

    fn fetch_universe(&self, valuation_date: NaiveDate) -> Result<Vec<ScreenerUniverseRecord>, String> {
        let registry = self.fetch_controller_registry(valuation_date)?.registry;
        validate_complete_universe(
            self.fetch_exchange_universe("sh", &registry)?,
            self.fetch_exchange_universe("sz", &registry)?,
            self.fetch_exchange_universe("bj", &registry)?,
        )
    }

    fn fetch_dividends(
        &self,
        _code: &str,
        _exchange: &str,
        _year: i32,
        _valuation_date: NaiveDate,
    ) -> Result<Vec<NormalizedDividend>, String> {
        Err("Eastmoney dividend transport is not wired yet".into())
    }

    fn fetch_quote(&self, _code: &str, _exchange: &str) -> Result<NormalizedQuote, String> {
        Err("Eastmoney quote transport is not wired yet".into())
    }

    fn fetch_klines(
        &self,
        _code: &str,
        _exchange: &str,
        _period: ScreenerPeriod,
    ) -> Result<Vec<NormalizedKline>, String> {
        Err("Eastmoney kline transport is not wired yet".into())
    }
}

fn parse_universe_line(
    expected_exchange: &str,
    line: &str,
    controllers: &mut HashMap<(String, String), String>,
) -> Result<EastmoneyUniverseRow, String> {
    let parts = line.split('|').collect::<Vec<_>>();
    if parts.len() != 6 {
        return Err("universe row must contain six fields".into());
    }
    let code = parts[0].trim().to_string();
    let name = parts[1].trim().to_string();
    let market_cap_cny = parts[2]
        .trim()
        .parse::<f64>()
        .map_err(|error| error.to_string())?;
    let exchange = parts[3].trim().to_string();
    if exchange != expected_exchange {
        return Err("universe row exchange mismatch".into());
    }
    let eastmoney_secid = parts[4].trim().to_string();
    controllers.insert((code.clone(), exchange.clone()), parts[5].trim().to_string());

    Ok(EastmoneyUniverseRow {
        code,
        name,
        market_cap_cny,
        market: exchange,
        eastmoney_secid,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn naive_date(date: &str) -> NaiveDate {
        NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap()
    }

    fn fake_eastmoney_getter() -> impl Fn(&str) -> Result<String, String> + Clone {
        |key| match key {
            "universe:sh" => Ok("600001|测试上海|50000000000|sh|1.600001|中国移动通信集团有限公司".into()),
            "universe:sz" => Ok("000001|测试深圳|50000000000|sz|0.000001|民营企业".into()),
            "universe:bj" => Ok("830001|测试北交|50000000000|bj|0.830001|某市国资委".into()),
            _ => Err(format!("unexpected eastmoney key {key}")),
        }
    }

    fn fake_sasac_getter() -> impl Fn(&str) -> Result<String, String> + Clone {
        |_| Ok(r#"<article><a>中国移动通信集团有限公司</a></article>"#.into())
    }

    #[test]
    fn concrete_source_combines_registry_universe_profile_and_market_clients() {
        let source = EastmoneyScreenerSource::with_getters(fake_eastmoney_getter(), fake_sasac_getter());
        let records = source.fetch_universe(naive_date("2026-07-21")).unwrap();
        assert_eq!(records[0].controller_classification, ControllerClassification::CentralSoe);
        assert_eq!(records[0].actual_controller, "中国移动通信集团有限公司");
    }
}
