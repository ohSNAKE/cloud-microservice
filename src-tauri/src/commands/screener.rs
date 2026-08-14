use crate::models::{
    ScreenerDashboard, ScreenerRefreshSchedule, ScreenerResult, ScreenerWatchlistAddResult,
};
use crate::services::screener::{
    dividend_yield, evaluate_boll_match, lower_boll_band, passes_fundamentals,
    trailing_cash_dividend, ScreenerPeriod,
};
use crate::{
    commands::screener_repository::{
        persist_completed_run, persist_failed_attempt, read_screener_dashboard, PendingScreenerRun,
    },
    services::screener_calendar::schedule_at,
    services::screener_source::{
        ControllerClassification, EastmoneyScreenerSource, ScreenerSource,
    },
    AppState,
};
use chrono::{Datelike, FixedOffset, TimeZone, Utc};
use rusqlite::{params, Connection};
use std::time::Duration;
use tauri::State;

pub fn add_screener_result_to_watchlist_in_conn(
    conn: &Connection,
    code: &str,
    exchange: &str,
    name: &str,
) -> Result<ScreenerWatchlistAddResult, String> {
    let code = code.trim();
    let exchange = exchange.trim();
    let name = name.trim();
    if code.is_empty() {
        return Err("代码不能为空".into());
    }
    if !matches!(exchange, "sh" | "sz" | "bj") {
        return Err("unsupported screener exchange".into());
    }
    let affected = conn
        .execute(
            "INSERT OR IGNORE INTO quant_watchlist (code, name, market, enabled)
             VALUES (?1, ?2, 'cn', 1)",
            params![code, if name.is_empty() { code } else { name }],
        )
        .map_err(|error| error.to_string())?;
    Ok(ScreenerWatchlistAddResult {
        added: affected == 1,
        already_present: affected == 0,
    })
}

#[tauri::command]
pub fn get_screener_dashboard(state: State<AppState>) -> Result<ScreenerDashboard, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    read_screener_dashboard(&conn)
}

#[tauri::command]
pub async fn refresh_screener(state: State<'_, AppState>) -> Result<ScreenerDashboard, String> {
    let valuation_date = Utc::now()
        .with_timezone(&FixedOffset::east_opt(8 * 3600).unwrap())
        .date_naive();

    // Run the network-heavy fetch off the UI thread and without holding the DB lock.
    let computed = tauri::async_runtime::spawn_blocking(move || {
        compute_screener_refresh(&default_screener_source(), valuation_date)
    })
    .await
    .map_err(|error| error.to_string())?;

    let conn = state.db.lock().map_err(|error| error.to_string())?;
    match computed {
        Ok(outcome) => persist_completed_run(&conn, outcome.run, outcome.results),
        Err(error) => persist_failed_attempt(
            &conn,
            "provider_unavailable",
            &format!("选股刷新失败：{error}"),
        ),
    }
}

pub fn refresh_screener_with_source<S: ScreenerSource>(
    conn: &Connection,
    source: &S,
    valuation_date: chrono::NaiveDate,
) -> Result<ScreenerDashboard, String> {
    run_screener_refresh_with_source(conn, source, valuation_date).or_else(|error| {
        persist_failed_attempt(
            conn,
            "provider_unavailable",
            &format!("选股刷新失败：{error}"),
        )
    })
}

pub struct ScreenerRefreshOutcome {
    pub run: PendingScreenerRun,
    pub results: Vec<ScreenerResult>,
    /// (code, exchange, actual_controller) discovered this run, for caching.
    pub controller_updates: Vec<(String, String, String)>,
}

pub fn run_screener_refresh_with_source<S: ScreenerSource>(
    conn: &Connection,
    source: &S,
    valuation_date: chrono::NaiveDate,
) -> Result<ScreenerDashboard, String> {
    let outcome = compute_screener_refresh(source, valuation_date)?;
    persist_completed_run(conn, outcome.run, outcome.results)
}

pub fn compute_screener_refresh<S: ScreenerSource>(
    source: &S,
    valuation_date: chrono::NaiveDate,
) -> Result<ScreenerRefreshOutcome, String> {
    let started_at = Utc::now().to_rfc3339();
    let china_tz = FixedOffset::east_opt(8 * 3600).unwrap();
    let now = china_tz
        .with_ymd_and_hms(
            valuation_date.year(),
            valuation_date.month(),
            valuation_date.day(),
            16,
            0,
            0,
        )
        .single()
        .ok_or_else(|| "invalid screener valuation date".to_string())?;
    let dividend_year = valuation_date.year() - 1;
    let universe = source.fetch_universe(valuation_date)?;
    let mut candidate_count = 0_i64;
    let mut skipped_count = 0_i64;
    let mut results = Vec::new();
    let controller_updates: Vec<(String, String, String)> = universe
        .iter()
        .filter(|record| !record.actual_controller.trim().is_empty())
        .map(|record| {
            (
                record.code.clone(),
                record.exchange.clone(),
                record.actual_controller.clone(),
            )
        })
        .collect();

    for record in universe {
        if !record.ordinary_equity
            || record.controller_classification != ControllerClassification::CentralSoe
            || record.market_cap_cny < 50_000_000_000.0
        {
            continue;
        }
        candidate_count += 1;

        let quote = match source.fetch_quote(&record.code, &record.exchange) {
            Ok(quote) => quote,
            Err(_) => {
                skipped_count += 1;
                continue;
            }
        };
        let dividends = match source.fetch_dividends(
            &record.code,
            &record.exchange,
            dividend_year,
            valuation_date,
        ) {
            Ok(dividends) => dividends,
            Err(_) => {
                skipped_count += 1;
                continue;
            }
        };
        let cash_dividend_per_share = match trailing_cash_dividend(&dividends, dividend_year) {
            Some(value) => value,
            None => continue,
        };
        let yield_value = match dividend_yield(&dividends, dividend_year, quote.price) {
            Some(value) => value,
            None => continue,
        };
        if !passes_fundamentals(record.market_cap_cny, yield_value) {
            continue;
        }

        let daily_bars =
            match source.fetch_klines(&record.code, &record.exchange, ScreenerPeriod::Day) {
                Ok(bars) => bars,
                Err(_) => {
                    skipped_count += 1;
                    continue;
                }
            };
        let weekly_bars =
            match source.fetch_klines(&record.code, &record.exchange, ScreenerPeriod::Week) {
                Ok(bars) => bars,
                Err(_) => {
                    skipped_count += 1;
                    continue;
                }
            };
        let daily_lower_band = lower_boll_band(&daily_bars, now, ScreenerPeriod::Day);
        let weekly_lower_band = lower_boll_band(&weekly_bars, now, ScreenerPeriod::Week);
        let boll_match = evaluate_boll_match(quote.price, daily_lower_band, weekly_lower_band);
        if boll_match.periods.is_empty() {
            continue;
        }

        results.push(ScreenerResult {
            code: record.code,
            exchange: record.exchange,
            name: record.name,
            market_cap_cny: record.market_cap_cny,
            cash_dividend_per_share,
            dividend_yield: yield_value,
            current_price: quote.price,
            price_observed_at: quote.observed_at.clone(),
            daily_lower_band,
            daily_distance: band_distance(quote.price, daily_lower_band),
            daily_kline_completed_at: latest_completed_at(&daily_bars),
            weekly_lower_band,
            weekly_distance: band_distance(quote.price, weekly_lower_band),
            weekly_kline_completed_at: latest_completed_at(&weekly_bars),
            matched_periods: boll_match.periods.into_iter().map(period_label).collect(),
            source_metadata: serde_json::json!({
                "eastmoney_secid": record.eastmoney_secid,
                "actual_controller": record.actual_controller,
            })
            .to_string(),
            fundamental_observed_at: quote.observed_at,
        });
    }

    let status = if skipped_count > 0 {
        "partial"
    } else {
        "success"
    };
    Ok(ScreenerRefreshOutcome {
        run: PendingScreenerRun {
            status: status.into(),
            started_at,
            completed_at: Utc::now().to_rfc3339(),
            candidate_count,
            match_count: results.len() as i64,
            skipped_count,
            failure_code: (skipped_count > 0).then_some("partial_provider_failure".into()),
            failure_message: (skipped_count > 0).then_some("部分标的数据获取失败".into()),
        },
        results,
        controller_updates,
    })
}

fn band_distance(price: f64, lower_band: Option<f64>) -> Option<f64> {
    lower_band
        .filter(|lower| *lower > 0.0)
        .map(|lower| price / lower - 1.0)
}

fn latest_completed_at(bars: &[crate::services::screener::NormalizedKline]) -> Option<String> {
    bars.iter().map(|bar| bar.completed_at.clone()).max()
}

fn period_label(period: ScreenerPeriod) -> String {
    match period {
        ScreenerPeriod::Day => "day".into(),
        ScreenerPeriod::Week => "week".into(),
    }
}

fn default_screener_source(
) -> EastmoneyScreenerSource<fn(&str) -> Result<String, String>, fn(&str) -> Result<String, String>>
{
    EastmoneyScreenerSource::with_getters(screener_http_get, screener_http_get)
}

fn screener_http_get(url: &str) -> Result<String, String> {
    if !url.starts_with("https://") {
        return Err(format!("unsupported screener source URL: {url}"));
    }
    ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(20))
        .build()
        .get(url)
        .set("User-Agent", "Mozilla/5.0")
        .set("Referer", "https://finance.eastmoney.com/")
        .call()
        .map_err(|error| error.to_string())?
        .into_string()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_screener_refresh_schedule() -> Result<ScreenerRefreshSchedule, String> {
    Ok(schedule_at(Utc::now().fixed_offset()))
}

#[tauri::command]
pub fn add_screener_result_to_watchlist(
    state: State<AppState>,
    code: String,
    exchange: String,
    name: String,
) -> Result<ScreenerWatchlistAddResult, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    add_screener_result_to_watchlist_in_conn(&conn, &code, &exchange, &name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::ensure_screener_schema;
    use crate::services::screener::{
        CentralControllerRegistry, DividendAdjustment, NormalizedDividend, NormalizedKline,
        ScreenerPeriod,
    };
    use crate::services::screener_source::{
        CentralControllerRegistrySnapshot, ControllerClassification, NormalizedQuote,
        ScreenerSource, ScreenerUniverseRecord,
    };
    use chrono::{Datelike, NaiveDate};
    use rusqlite::Connection;

    fn setup_quant_watchlist_schema() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE quant_watchlist (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                code TEXT NOT NULL,
                name TEXT NOT NULL,
                market TEXT NOT NULL DEFAULT 'cn',
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
                UNIQUE(code, market)
            );",
        )
        .unwrap();
        conn
    }

    #[test]
    fn adding_existing_screener_result_to_cn_watchlist_is_successful_noop() {
        let conn = setup_quant_watchlist_schema();
        conn.execute(
            "INSERT INTO quant_watchlist (code, name, market, enabled) VALUES ('600001', '原名', 'cn', 0)",
            [],
        )
        .unwrap();

        let result =
            add_screener_result_to_watchlist_in_conn(&conn, "600001", "sh", "测试").unwrap();

        assert!(!result.added);
        assert!(result.already_present);
        let existing: (String, i64) = conn
            .query_row(
                "SELECT name, enabled FROM quant_watchlist WHERE code = '600001' AND market = 'cn'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(existing, ("原名".to_string(), 0));
    }

    #[test]
    fn screener_watchlist_add_accepts_sh_sz_and_bj_only() {
        let conn = setup_quant_watchlist_schema();
        assert!(
            add_screener_result_to_watchlist_in_conn(&conn, "600001", "sh", "上证")
                .unwrap()
                .added
        );
        assert!(
            add_screener_result_to_watchlist_in_conn(&conn, "000001", "sz", "深证")
                .unwrap()
                .added
        );
        assert!(
            add_screener_result_to_watchlist_in_conn(&conn, "830001", "bj", "北证")
                .unwrap()
                .added
        );
        assert!(add_screener_result_to_watchlist_in_conn(&conn, "00700", "hk", "腾讯").is_err());
    }

    struct FakeMatchingSource;

    impl ScreenerSource for FakeMatchingSource {
        fn fetch_controller_registry(
            &self,
            valuation_date: NaiveDate,
        ) -> Result<CentralControllerRegistrySnapshot, String> {
            Ok(CentralControllerRegistrySnapshot {
                valuation_date,
                registry: CentralControllerRegistry::from_entries(vec![]),
                source_url: "test".into(),
                source_date: valuation_date.to_string(),
            })
        }

        fn fetch_universe(
            &self,
            _valuation_date: NaiveDate,
        ) -> Result<Vec<ScreenerUniverseRecord>, String> {
            Ok(vec![ScreenerUniverseRecord {
                code: "600001".into(),
                exchange: "sh".into(),
                name: "测试央企".into(),
                eastmoney_secid: "1.600001".into(),
                ordinary_equity: true,
                market_cap_cny: 60_000_000_000.0,
                actual_controller: "国务院国有资产监督管理委员会".into(),
                controller_classification: ControllerClassification::CentralSoe,
            }])
        }

        fn fetch_dividends(
            &self,
            code: &str,
            exchange: &str,
            _year: i32,
            valuation_date: NaiveDate,
        ) -> Result<Vec<NormalizedDividend>, String> {
            Ok(vec![NormalizedDividend {
                code: code.into(),
                exchange: exchange.into(),
                ex_dividend_date: NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
                source_cash_per_ten_shares: Some(6.0),
                ex_date_total_capital: None,
                valuation_date_total_capital: None,
                gross_per_current_share: 0.6,
                adjustment: DividendAdjustment::VerifiedUnadjusted,
                confirmed_cash: valuation_date.year() == 2026,
            }])
        }

        fn fetch_quote(&self, code: &str, exchange: &str) -> Result<NormalizedQuote, String> {
            Ok(NormalizedQuote {
                code: code.into(),
                exchange: exchange.into(),
                price: 10.1,
                observed_at: "2026-07-21T16:00:00+08:00".into(),
            })
        }

        fn fetch_klines(
            &self,
            code: &str,
            exchange: &str,
            period: ScreenerPeriod,
        ) -> Result<Vec<NormalizedKline>, String> {
            Ok((1..=20)
                .map(|day| NormalizedKline {
                    code: code.into(),
                    exchange: exchange.into(),
                    period,
                    trading_date: NaiveDate::from_ymd_opt(2026, 6, day).unwrap(),
                    completed_at: format!("2026-06-{day:02}T15:00:00+08:00"),
                    close: 10.0,
                })
                .collect())
        }
    }

    struct FailingSource;

    impl ScreenerSource for FailingSource {
        fn fetch_controller_registry(
            &self,
            valuation_date: NaiveDate,
        ) -> Result<CentralControllerRegistrySnapshot, String> {
            Ok(CentralControllerRegistrySnapshot {
                valuation_date,
                registry: CentralControllerRegistry::from_entries(vec![]),
                source_url: "test".into(),
                source_date: valuation_date.to_string(),
            })
        }

        fn fetch_universe(
            &self,
            _valuation_date: NaiveDate,
        ) -> Result<Vec<ScreenerUniverseRecord>, String> {
            Err("upstream timeout".into())
        }

        fn fetch_dividends(
            &self,
            _code: &str,
            _exchange: &str,
            _year: i32,
            _valuation_date: NaiveDate,
        ) -> Result<Vec<NormalizedDividend>, String> {
            unreachable!()
        }

        fn fetch_quote(&self, _code: &str, _exchange: &str) -> Result<NormalizedQuote, String> {
            unreachable!()
        }

        fn fetch_klines(
            &self,
            _code: &str,
            _exchange: &str,
            _period: ScreenerPeriod,
        ) -> Result<Vec<NormalizedKline>, String> {
            unreachable!()
        }
    }

    #[test]
    fn compute_screener_refresh_returns_run_and_results_without_db() {
        let outcome = compute_screener_refresh(
            &FakeMatchingSource,
            NaiveDate::from_ymd_opt(2026, 7, 21).unwrap(),
        )
        .unwrap();

        assert_eq!(outcome.run.status, "success");
        assert_eq!(outcome.run.candidate_count, 1);
        assert_eq!(outcome.results.len(), 1);
        assert_eq!(outcome.results[0].code, "600001");
        assert!(outcome
            .controller_updates
            .iter()
            .any(|(code, exchange, controller)| code == "600001"
                && exchange == "sh"
                && controller == "国务院国有资产监督管理委员会"));
    }

    #[test]
    fn refresh_with_source_persists_matching_results() {
        let conn = Connection::open_in_memory().unwrap();
        ensure_screener_schema(&conn).unwrap();

        let dashboard = run_screener_refresh_with_source(
            &conn,
            &FakeMatchingSource,
            NaiveDate::from_ymd_opt(2026, 7, 21).unwrap(),
        )
        .unwrap();

        assert_eq!(dashboard.displayed_run.unwrap().status, "success");
        assert_eq!(dashboard.results.len(), 1);
        assert_eq!(dashboard.results[0].code, "600001");
        assert_eq!(dashboard.results[0].matched_periods, vec!["day", "week"]);
    }

    #[test]
    fn refresh_with_source_failure_persists_failed_attempt() {
        let conn = Connection::open_in_memory().unwrap();
        ensure_screener_schema(&conn).unwrap();

        let dashboard = refresh_screener_with_source(
            &conn,
            &FailingSource,
            NaiveDate::from_ymd_opt(2026, 7, 21).unwrap(),
        )
        .unwrap();

        let failure = dashboard
            .latest_failed_attempt
            .unwrap()
            .failure_summary
            .unwrap();
        assert_eq!(failure.code, "provider_unavailable");
        assert!(failure.message.contains("upstream timeout"));
    }

    #[test]
    fn refresh_counts_only_central_large_cap_candidates() {
        let conn = Connection::open_in_memory().unwrap();
        ensure_screener_schema(&conn).unwrap();

        let dashboard = run_screener_refresh_with_source(
            &conn,
            &FakeMixedUniverseSource,
            NaiveDate::from_ymd_opt(2026, 7, 21).unwrap(),
        )
        .unwrap();

        let run = dashboard.displayed_run.unwrap();
        assert_eq!(run.candidate_count, 1);
        assert_eq!(run.skipped_count, 0);
    }

    struct FakeMixedUniverseSource;

    impl ScreenerSource for FakeMixedUniverseSource {
        fn fetch_controller_registry(
            &self,
            valuation_date: NaiveDate,
        ) -> Result<CentralControllerRegistrySnapshot, String> {
            FakeMatchingSource.fetch_controller_registry(valuation_date)
        }

        fn fetch_universe(
            &self,
            valuation_date: NaiveDate,
        ) -> Result<Vec<ScreenerUniverseRecord>, String> {
            let mut records = FakeMatchingSource.fetch_universe(valuation_date)?;
            records.push(ScreenerUniverseRecord {
                code: "600002".into(),
                exchange: "sh".into(),
                name: "小市值央企".into(),
                eastmoney_secid: "1.600002".into(),
                ordinary_equity: true,
                market_cap_cny: 49_000_000_000.0,
                actual_controller: "国务院国有资产监督管理委员会".into(),
                controller_classification: ControllerClassification::CentralSoe,
            });
            records.push(ScreenerUniverseRecord {
                code: "600003".into(),
                exchange: "sh".into(),
                name: "地方国企".into(),
                eastmoney_secid: "1.600003".into(),
                ordinary_equity: true,
                market_cap_cny: 60_000_000_000.0,
                actual_controller: "某市国资委".into(),
                controller_classification: ControllerClassification::LocalSoe,
            });
            Ok(records)
        }

        fn fetch_dividends(
            &self,
            code: &str,
            exchange: &str,
            year: i32,
            valuation_date: NaiveDate,
        ) -> Result<Vec<NormalizedDividend>, String> {
            FakeMatchingSource.fetch_dividends(code, exchange, year, valuation_date)
        }

        fn fetch_quote(&self, code: &str, exchange: &str) -> Result<NormalizedQuote, String> {
            assert_eq!(code, "600001");
            FakeMatchingSource.fetch_quote(code, exchange)
        }

        fn fetch_klines(
            &self,
            code: &str,
            exchange: &str,
            period: ScreenerPeriod,
        ) -> Result<Vec<NormalizedKline>, String> {
            FakeMatchingSource.fetch_klines(code, exchange, period)
        }
    }
}
