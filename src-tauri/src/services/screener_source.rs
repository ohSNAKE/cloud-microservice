use crate::services::screener::{
    CentralControllerRegistry, NormalizedDividend, NormalizedKline, ScreenerPeriod,
};
use crate::services::screener_source_eastmoney_market::{
    build_dividend_url, build_kline_url, build_quote_url, parse_dividend_json, parse_kline_json,
    parse_quote_json,
};
use crate::services::screener_source_eastmoney_universe::{
    build_actual_controller_url, build_universe_page_url, classify_controller,
    collect_exchange_pages, parse_actual_controller_json, parse_universe_page_json,
    validate_complete_universe,
};
use crate::services::screener_source_sasac::{
    embedded_central_controller_entries, parse_sasac_directory, SASAC_DIRECTORY_URL,
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

    fn fetch_universe(
        &self,
        valuation_date: NaiveDate,
    ) -> Result<Vec<ScreenerUniverseRecord>, String>;

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
    controller_cache: std::collections::HashMap<(String, String), String>,
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
            controller_cache: std::collections::HashMap::new(),
        }
    }

    pub fn with_getters_and_controller_cache(
        eastmoney_getter: EastmoneyGetter,
        sasac_getter: SasacGetter,
        controller_cache: std::collections::HashMap<(String, String), String>,
    ) -> Self {
        Self {
            eastmoney_getter,
            sasac_getter,
            controller_cache,
        }
    }

    fn fetch_exchange_universe(
        &self,
        exchange: &str,
        registry: &CentralControllerRegistry,
    ) -> Result<Vec<ScreenerUniverseRecord>, String> {
        let page_size = 100;
        let first_url = build_universe_page_url(exchange, 1, page_size)?;
        let first_payload = (self.eastmoney_getter)(&first_url)?;
        let first_page = parse_universe_page_json(exchange, 1, page_size, &first_payload)?;
        let total_pages = first_page.total_pages;
        let mut pages = vec![first_page];
        for page_no in 2..=total_pages {
            let url = build_universe_page_url(exchange, page_no, page_size)?;
            let payload = (self.eastmoney_getter)(&url)?;
            pages.push(parse_universe_page_json(
                exchange, page_no, page_size, &payload,
            )?);
        }
        let mut records = collect_exchange_pages(exchange, pages)?;
        for record in &mut records {
            if record.market_cap_cny < 50_000_000_000.0 {
                continue;
            }
            let cache_key = (record.code.clone(), record.exchange.clone());
            let actual_controller = match self.controller_cache.get(&cache_key) {
                Some(cached) => cached.clone(),
                None => build_actual_controller_url(&record.code, &record.exchange)
                    .and_then(|url| (self.eastmoney_getter)(&url))
                    .and_then(|payload| parse_actual_controller_json(&payload))
                    .unwrap_or_default(),
            };
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
        match (self.sasac_getter)(SASAC_DIRECTORY_URL)
            .and_then(|html| parse_sasac_directory(&valuation_date.to_string(), &html))
        {
            Ok(snapshot) => Ok(CentralControllerRegistrySnapshot {
                valuation_date,
                registry: CentralControllerRegistry::from_entries(snapshot.entries),
                source_url: snapshot.source_url,
                source_date: snapshot.source_date,
            }),
            Err(_) => Ok(CentralControllerRegistrySnapshot {
                valuation_date,
                registry: CentralControllerRegistry::from_entries(
                    embedded_central_controller_entries(),
                ),
                source_url: "embedded:sasac-central-enterprises".into(),
                source_date: valuation_date.to_string(),
            }),
        }
    }

    fn fetch_universe(
        &self,
        valuation_date: NaiveDate,
    ) -> Result<Vec<ScreenerUniverseRecord>, String> {
        let registry = self.fetch_controller_registry(valuation_date)?.registry;
        validate_complete_universe(
            self.fetch_exchange_universe("sh", &registry)?,
            self.fetch_exchange_universe("sz", &registry)?,
            self.fetch_exchange_universe("bj", &registry)?,
        )
    }

    fn fetch_dividends(
        &self,
        code: &str,
        exchange: &str,
        _year: i32,
        _valuation_date: NaiveDate,
    ) -> Result<Vec<NormalizedDividend>, String> {
        let payload = (self.eastmoney_getter)(&build_dividend_url(code))?;
        parse_dividend_json(code, exchange, &payload)
    }

    fn fetch_quote(&self, code: &str, exchange: &str) -> Result<NormalizedQuote, String> {
        let url = build_quote_url(code, exchange)?;
        let payload = (self.eastmoney_getter)(&url)?;
        parse_quote_json(code, exchange, &payload)
    }

    fn fetch_klines(
        &self,
        code: &str,
        exchange: &str,
        period: ScreenerPeriod,
    ) -> Result<Vec<NormalizedKline>, String> {
        let url = build_kline_url(code, exchange, period)?;
        let payload = (self.eastmoney_getter)(&url)?;
        parse_kline_json(code, exchange, period, &payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn naive_date(date: &str) -> NaiveDate {
        NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap()
    }

    fn fake_eastmoney_getter() -> impl Fn(&str) -> Result<String, String> + Clone {
        |url| {
            if url.contains("fs=m:1+t:2") {
                return Ok(r#"{"data":{"total":1,"diff":[{"f12":"600001","f14":"测试上海","f20":50000000000}]}}"#.into());
            }
            if url.contains("fs=m:0+t:6") {
                return Ok(r#"{"data":{"total":1,"diff":[{"f12":"000001","f14":"测试深圳","f20":50000000000}]}}"#.into());
            }
            if url.contains("fs=m:0+t:81") {
                return Ok(r#"{"data":{"total":1,"diff":[{"f12":"830001","f14":"测试北交","f20":50000000000}]}}"#.into());
            }
            if url.contains("SECUCODE%3D%22600001.SH%22") {
                return Ok(
                    r#"{"result":{"data":[{"ACTUAL_HOLDER":"中国移动通信集团有限公司"}]}}"#.into(),
                );
            }
            if url.contains("SECUCODE%3D%22000001.SZ%22") {
                return Ok(r#"{"result":{"data":[{"ACTUAL_HOLDER":"民营企业"}]}}"#.into());
            }
            if url.contains("SECUCODE%3D%22830001.BJ%22") {
                return Ok(r#"{"result":{"data":[{"ACTUAL_HOLDER":"某市国资委"}]}}"#.into());
            }
            if url.contains("stock/get?secid=1.600001") {
                return Ok(r#"{"data":{"f43":1010,"f124":1784707200}}"#.into());
            }
            if url.contains("kline/get?secid=1.600001") {
                return Ok(r#"{"data":{"klines":["2026-07-01,10,10,10,10,1"]}}"#.into());
            }
            if url.contains("RPT_SHAREBONUS_DET") {
                return Ok(r#"{"result":{"data":[{"PRETAX_BONUS_RMB":6.0,"EX_DIVIDEND_DATE":"2025-06-01 00:00:00","ASSIGN_PROGRESS":"实施分配"}]}}"#.into());
            }
            Err(format!("unexpected eastmoney URL {url}"))
        }
    }

    fn fake_sasac_getter() -> impl Fn(&str) -> Result<String, String> + Clone {
        |_| Ok(r#"<article><a>中国移动通信集团有限公司</a></article>"#.into())
    }

    fn failing_sasac_getter() -> impl Fn(&str) -> Result<String, String> + Clone {
        |_| Err("sasac unavailable".into())
    }

    #[test]
    fn concrete_source_combines_registry_universe_profile_and_market_clients() {
        let source =
            EastmoneyScreenerSource::with_getters(fake_eastmoney_getter(), fake_sasac_getter());
        let records = source.fetch_universe(naive_date("2026-07-21")).unwrap();
        let central = records
            .iter()
            .find(|record| record.code == "600001")
            .unwrap();
        assert_eq!(
            central.controller_classification,
            ControllerClassification::CentralSoe
        );
        assert_eq!(central.actual_controller, "中国移动通信集团有限公司");
    }

    #[test]
    fn concrete_source_fetches_market_records() {
        let source =
            EastmoneyScreenerSource::with_getters(fake_eastmoney_getter(), fake_sasac_getter());

        assert_eq!(source.fetch_quote("600001", "sh").unwrap().price, 10.1);
        assert_eq!(
            source
                .fetch_klines("600001", "sh", ScreenerPeriod::Day)
                .unwrap()[0]
                .close,
            10.0
        );
        assert_eq!(
            source
                .fetch_dividends("600001", "sh", 2025, naive_date("2026-07-21"))
                .unwrap()[0]
                .gross_per_current_share,
            0.6
        );
    }

    #[test]
    fn concrete_source_falls_back_to_embedded_controller_registry() {
        let source =
            EastmoneyScreenerSource::with_getters(fake_eastmoney_getter(), failing_sasac_getter());

        let records = source.fetch_universe(naive_date("2026-07-21")).unwrap();
        let central = records
            .iter()
            .find(|record| record.code == "600001")
            .unwrap();

        assert_eq!(
            central.controller_classification,
            ControllerClassification::CentralSoe
        );
    }

    fn controller_url_forbidden_getter() -> impl Fn(&str) -> Result<String, String> + Clone {
        |url: &str| {
            if url.contains("SECUCODE") {
                return Err("controller URL must not be fetched when cached".into());
            }
            fake_eastmoney_getter()(url)
        }
    }

    #[test]
    fn cached_controller_is_used_without_fetching_controller_url() {
        let mut cache = std::collections::HashMap::new();
        cache.insert(
            ("600001".to_string(), "sh".to_string()),
            "中国移动通信集团有限公司".to_string(),
        );
        let source = EastmoneyScreenerSource::with_getters_and_controller_cache(
            controller_url_forbidden_getter(),
            fake_sasac_getter(),
            cache,
        );

        let records = source.fetch_universe(naive_date("2026-07-21")).unwrap();
        let central = records
            .iter()
            .find(|record| record.code == "600001")
            .unwrap();

        assert_eq!(central.actual_controller, "中国移动通信集团有限公司");
        assert_eq!(
            central.controller_classification,
            ControllerClassification::CentralSoe
        );
    }
}
