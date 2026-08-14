use chrono::{DateTime, Datelike, FixedOffset, NaiveDate, NaiveTime};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use unicode_normalization::UnicodeNormalization;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScreenerPeriod {
    Day,
    Week,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DividendAdjustment {
    Adjusted,
    VerifiedUnadjusted,
    Unverifiable,
}

#[derive(Debug, Clone)]
pub struct NormalizedDividend {
    pub code: String,
    pub exchange: String,
    pub ex_dividend_date: NaiveDate,
    pub source_cash_per_ten_shares: Option<f64>,
    pub ex_date_total_capital: Option<f64>,
    pub valuation_date_total_capital: Option<f64>,
    pub gross_per_current_share: f64,
    pub adjustment: DividendAdjustment,
    pub confirmed_cash: bool,
}

#[derive(Debug, Clone)]
pub struct CentralControllerEntry {
    pub original_name: String,
    pub normalized_name: String,
    pub aliases: Vec<String>,
}

impl CentralControllerEntry {
    pub fn new(original_name: &str, aliases: Vec<String>) -> Self {
        Self {
            original_name: original_name.to_string(),
            normalized_name: normalize_controller_name(original_name),
            aliases: aliases
                .into_iter()
                .map(|alias| normalize_controller_name(&alias))
                .collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CentralControllerRegistry {
    pub valuation_date: NaiveDate,
    pub entries: HashSet<String>,
}

impl CentralControllerRegistry {
    pub fn from_entries(entries: Vec<CentralControllerEntry>) -> Self {
        let mut normalized_entries = HashSet::new();
        for entry in entries {
            normalized_entries.insert(entry.normalized_name);
            normalized_entries.extend(entry.aliases);
        }

        Self {
            valuation_date: NaiveDate::from_ymd_opt(1970, 1, 1).unwrap(),
            entries: normalized_entries,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BollMatch {
    pub periods: Vec<ScreenerPeriod>,
    pub daily_lower_band: Option<f64>,
    pub weekly_lower_band: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct NormalizedKline {
    pub code: String,
    pub exchange: String,
    pub period: ScreenerPeriod,
    pub trading_date: NaiveDate,
    pub completed_at: String,
    pub close: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScreenerCandidateMatch {
    pub code: String,
    pub exchange: String,
    pub name: String,
    pub distance: f64,
}

pub fn normalize_controller_name(value: &str) -> String {
    let normalized: String = value.nfkc().collect();
    let trimmed = normalized.trim();
    ["（集团）有限公司", "集团有限公司", "有限公司", "集团"]
        .iter()
        .find_map(|suffix| trimmed.strip_suffix(suffix))
        .unwrap_or(trimmed)
        .trim()
        .to_string()
}

pub fn is_central_soe(value: &str, registry: &CentralControllerRegistry) -> bool {
    let normalized = normalize_controller_name(value);
    normalized == "国务院"
        || normalized == "国务院国有资产监督管理委员会"
        || registry.entries.contains(&normalized)
}

pub fn trailing_cash_dividend(records: &[NormalizedDividend], year: i32) -> Option<f64> {
    let matching = records
        .iter()
        .filter(|record| record.ex_dividend_date.year() == year)
        .collect::<Vec<_>>();
    if matching.iter().any(|record| {
        record.confirmed_cash && record.adjustment == DividendAdjustment::Unverifiable
    }) {
        return None;
    }

    let total = matching
        .into_iter()
        .filter(|record| {
            record.confirmed_cash && record.adjustment != DividendAdjustment::Unverifiable
        })
        .map(|record| record.gross_per_current_share)
        .sum::<f64>();
    (total.is_finite() && total > 0.0).then_some(total)
}

pub fn dividend_yield(records: &[NormalizedDividend], year: i32, price: f64) -> Option<f64> {
    if !price.is_finite() || price <= 0.0 {
        return None;
    }
    trailing_cash_dividend(records, year).map(|dividend| dividend / price)
}

pub fn passes_fundamentals(market_cap_cny: f64, dividend_yield: f64) -> bool {
    market_cap_cny.is_finite()
        && dividend_yield.is_finite()
        && market_cap_cny >= 50_000_000_000.0
        && dividend_yield >= 0.05
}

pub fn completed_screener_bars(
    bars: &[NormalizedKline],
    now: DateTime<FixedOffset>,
    period: ScreenerPeriod,
) -> Vec<NormalizedKline> {
    let china_now = now.with_timezone(&FixedOffset::east_opt(8 * 3600).unwrap());
    let valuation_date = china_now.date_naive();
    let cutoff = NaiveTime::from_hms_opt(15, 0, 0).unwrap();

    bars.iter()
        .filter(|bar| bar.period == period && bar.trading_date <= valuation_date)
        .filter(|bar| match period {
            ScreenerPeriod::Day => bar.trading_date < valuation_date || china_now.time() >= cutoff,
            ScreenerPeriod::Week => completed_at(bar)
                .map(|completed_at| completed_at <= china_now)
                .unwrap_or(false),
        })
        .cloned()
        .collect()
}

pub fn validate_completed_bars(
    bars: &[NormalizedKline],
    period: ScreenerPeriod,
) -> Result<(), String> {
    let mut seen_dates = HashSet::new();
    let mut previous_completed_at: Option<DateTime<FixedOffset>> = None;

    for bar in bars {
        if bar.period != period {
            return Err("bar period mismatch".into());
        }
        if !bar.close.is_finite() || bar.close <= 0.0 {
            return Err("bar close must be positive and finite".into());
        }
        if !seen_dates.insert(bar.trading_date) {
            return Err("duplicate bar trading date".into());
        }
        let completed_at = completed_at(bar)?;
        if previous_completed_at
            .map(|previous| completed_at <= previous)
            .unwrap_or(false)
        {
            return Err("bars must be strictly ascending".into());
        }
        previous_completed_at = Some(completed_at);
    }

    Ok(())
}

pub fn lower_boll_band(
    bars: &[NormalizedKline],
    now: DateTime<FixedOffset>,
    period: ScreenerPeriod,
) -> Option<f64> {
    let china_now = now.with_timezone(&FixedOffset::east_opt(8 * 3600).unwrap());
    let completed = completed_screener_bars(bars, china_now, period);
    validate_completed_bars(&completed, period).ok()?;
    let closes = completed
        .iter()
        .rev()
        .take(20)
        .map(|bar| bar.close)
        .collect::<Vec<_>>();
    if closes.len() != 20 {
        return None;
    }

    let mean = closes.iter().sum::<f64>() / 20.0;
    let deviation = (closes
        .iter()
        .map(|close| (close - mean).powi(2))
        .sum::<f64>()
        / 20.0)
        .sqrt();
    let lower = mean - 2.0 * deviation;
    (lower.is_finite() && lower > 0.0).then_some(lower)
}

pub fn evaluate_boll_match(
    current_price: f64,
    daily_lower_band: Option<f64>,
    weekly_lower_band: Option<f64>,
) -> BollMatch {
    let mut periods = Vec::new();
    if period_matches(current_price, daily_lower_band) {
        periods.push(ScreenerPeriod::Day);
    }
    if period_matches(current_price, weekly_lower_band) {
        periods.push(ScreenerPeriod::Week);
    }

    BollMatch {
        periods,
        daily_lower_band,
        weekly_lower_band,
    }
}

pub fn sort_matches(mut matches: Vec<ScreenerCandidateMatch>) -> Vec<ScreenerCandidateMatch> {
    matches.sort_by(|left, right| {
        left.distance
            .total_cmp(&right.distance)
            .then_with(|| left.code.cmp(&right.code))
            .then_with(|| left.exchange.cmp(&right.exchange))
    });
    matches
}

fn period_matches(current_price: f64, lower_band: Option<f64>) -> bool {
    current_price.is_finite()
        && current_price > 0.0
        && lower_band
            .filter(|lower| lower.is_finite() && *lower > 0.0)
            .map(|lower| current_price <= lower * 1.02)
            .unwrap_or(false)
}

fn completed_at(bar: &NormalizedKline) -> Result<DateTime<FixedOffset>, String> {
    DateTime::parse_from_rfc3339(&bar.completed_at).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Duration, FixedOffset, NaiveDate, TimeZone};

    fn naive_date(date: &str) -> NaiveDate {
        NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap()
    }

    fn china_datetime(
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        minute: u32,
        second: u32,
    ) -> DateTime<FixedOffset> {
        FixedOffset::east_opt(8 * 3600)
            .unwrap()
            .with_ymd_and_hms(year, month, day, hour, minute, second)
            .single()
            .unwrap()
    }

    fn dividend(date: &str, amount: f64, adjustment: DividendAdjustment) -> NormalizedDividend {
        NormalizedDividend {
            code: "600001".into(),
            exchange: "sh".into(),
            ex_dividend_date: naive_date(date),
            source_cash_per_ten_shares: Some(amount * 10.0),
            ex_date_total_capital: None,
            valuation_date_total_capital: None,
            gross_per_current_share: amount,
            adjustment,
            confirmed_cash: true,
        }
    }

    fn unconfirmed_cash() -> NormalizedDividend {
        NormalizedDividend {
            confirmed_cash: false,
            ..dividend("2025-06-01", 0.5, DividendAdjustment::VerifiedUnadjusted)
        }
    }

    fn stock_dividend() -> NormalizedDividend {
        NormalizedDividend {
            gross_per_current_share: 0.0,
            confirmed_cash: false,
            ..dividend("2025-07-01", 0.0, DividendAdjustment::VerifiedUnadjusted)
        }
    }

    fn missing_date() -> NaiveDate {
        NaiveDate::from_ymd_opt(1900, 1, 1).unwrap()
    }

    fn missing_ex_date() -> NormalizedDividend {
        NormalizedDividend {
            ex_dividend_date: missing_date(),
            confirmed_cash: false,
            ..dividend("2025-08-01", 0.4, DividendAdjustment::VerifiedUnadjusted)
        }
    }

    fn bar(date: &str, close: f64) -> NormalizedKline {
        NormalizedKline {
            code: "600001".into(),
            exchange: "sh".into(),
            period: ScreenerPeriod::Day,
            trading_date: naive_date(date),
            completed_at: format!("{date}T15:00:00+08:00"),
            close,
        }
    }

    fn test_bars(count: usize, close: f64) -> Vec<NormalizedKline> {
        (0..count)
            .map(|index| bar(&format!("2026-07-{:02}", index + 1), close))
            .collect()
    }

    fn daily_bars_including(date: &str, close: f64) -> Vec<NormalizedKline> {
        let mut bars = test_bars(19, 10.0);
        bars.push(bar(date, close));
        bars
    }

    fn weekly_bars_including(date: &str, close: f64) -> Vec<NormalizedKline> {
        let included_date = naive_date(date);
        let first_week = included_date - Duration::days(20 * 7);
        let mut bars = (0..20)
            .map(|index| {
                let date = first_week + Duration::days(index as i64 * 7);
                let mut item = bar(&date.format("%Y-%m-%d").to_string(), 10.0);
                item.period = ScreenerPeriod::Week;
                item.completed_at = format!("{}T15:00:00+08:00", item.trading_date);
                item
            })
            .collect::<Vec<_>>();
        let mut latest = bar(date, close);
        latest.period = ScreenerPeriod::Week;
        latest.completed_at = format!("{date}T15:00:00+08:00");
        bars.push(latest);
        bars
    }

    fn match_row(code: &str, exchange: &str, distance: f64) -> ScreenerCandidateMatch {
        ScreenerCandidateMatch {
            code: code.into(),
            exchange: exchange.into(),
            name: "测试".into(),
            distance,
        }
    }

    #[test]
    fn central_controller_match_is_exact_after_allowed_suffix_normalization() {
        let registry = CentralControllerRegistry::from_entries(vec![CentralControllerEntry::new(
            "中国移动通信集团有限公司",
            vec!["中国移动".into()],
        )]);

        assert!(is_central_soe("中国移动通信集团", &registry));
        assert!(is_central_soe("国务院国有资产监督管理委员会", &registry));
        assert!(is_central_soe("国务院", &registry));
        assert!(!is_central_soe(
            "某地方国资委控股的中国移动通信集团",
            &registry
        ));
    }

    #[test]
    fn dividend_yield_uses_gross_current_share_dividends_from_prior_calendar_year() {
        let dividends = vec![
            dividend("2025-06-01", 0.5, DividendAdjustment::Adjusted),
            dividend("2025-12-20", 0.7, DividendAdjustment::VerifiedUnadjusted),
        ];

        assert_eq!(trailing_cash_dividend(&dividends, 2025), Some(1.2));
        assert_eq!(dividend_yield(&dividends, 2025, 20.0), Some(0.06));
    }

    #[test]
    fn unverifiable_confirmed_cash_dividend_invalidates_the_stock() {
        let dividends = vec![dividend(
            "2025-06-01",
            0.5,
            DividendAdjustment::Unverifiable,
        )];
        assert_eq!(trailing_cash_dividend(&dividends, 2025), None);
    }

    #[test]
    fn fundamental_thresholds_include_exact_fifty_billion_and_five_percent() {
        assert!(passes_fundamentals(50_000_000_000.0, 0.05));
        assert!(!passes_fundamentals(49_999_999_999.99, 0.05));
        assert!(!passes_fundamentals(50_000_000_000.0, 0.049_999));
    }

    #[test]
    fn unconfirmed_non_cash_or_missing_date_records_do_not_add_to_cash_dividend() {
        let dividends = vec![unconfirmed_cash(), stock_dividend(), missing_ex_date()];
        assert_eq!(trailing_cash_dividend(&dividends, 2025), None);
    }

    #[test]
    fn boll_uses_population_deviation_and_excludes_today_before_1500() {
        let mut bars = test_bars(20, 10.0);
        bars.push(bar("2026-07-21", 8.0));
        let now = china_datetime(2026, 7, 21, 14, 59, 59);

        let result = lower_boll_band(&bars, now, ScreenerPeriod::Day);
        assert_eq!(result, Some(10.0));
    }

    #[test]
    fn daily_bar_is_included_at_exactly_1500() {
        let bars = daily_bars_including("2026-07-21", 8.0);
        assert!(
            lower_boll_band(
                &bars,
                china_datetime(2026, 7, 21, 15, 0, 0),
                ScreenerPeriod::Day
            )
            .unwrap()
                < 10.0
        );
    }

    #[test]
    fn either_period_can_match_and_two_percent_boundary_is_inclusive() {
        let match_result = evaluate_boll_match(10.2, Some(10.0), Some(9.0));
        assert_eq!(match_result.periods, vec![ScreenerPeriod::Day]);
    }

    #[test]
    fn nonpositive_or_duplicate_bars_are_rejected_before_boll_calculation() {
        assert!(validate_completed_bars(&[bar("2026-07-20", 0.0)], ScreenerPeriod::Day).is_err());
        assert!(validate_completed_bars(
            &[bar("2026-07-20", 10.0), bar("2026-07-20", 11.0)],
            ScreenerPeriod::Day
        )
        .is_err());
    }

    #[test]
    fn result_order_uses_smallest_distance_then_code_then_exchange() {
        let ordered = sort_matches(vec![
            match_row("600001", "sh", 0.01),
            match_row("000001", "sz", 0.01),
        ]);
        assert_eq!(ordered[0].code, "000001");
    }

    #[test]
    fn both_or_one_available_period_can_match_but_neither_cannot() {
        assert_eq!(
            evaluate_boll_match(9.9, Some(10.0), Some(10.0)).periods,
            vec![ScreenerPeriod::Day, ScreenerPeriod::Week]
        );
        assert_eq!(
            evaluate_boll_match(10.1, Some(10.0), None).periods,
            vec![ScreenerPeriod::Day]
        );
        assert!(evaluate_boll_match(10.3, Some(10.0), Some(10.0))
            .periods
            .is_empty());
    }

    #[test]
    fn week_is_completed_only_after_friday_close() {
        let bars = weekly_bars_including("2026-07-24", 8.0);
        assert_eq!(
            lower_boll_band(
                &bars,
                china_datetime(2026, 7, 24, 14, 59, 59),
                ScreenerPeriod::Week
            ),
            Some(10.0)
        );
    }

    #[test]
    fn friday_bar_is_included_at_close_and_future_weekly_bars_are_excluded() {
        let bars = weekly_bars_including("2026-07-24", 8.0);
        assert!(
            lower_boll_band(
                &bars,
                china_datetime(2026, 7, 24, 15, 0, 0),
                ScreenerPeriod::Week
            )
            .unwrap()
                < 10.0
        );
        assert_eq!(
            completed_screener_bars(
                &weekly_bars_including("2026-07-31", 7.0),
                china_datetime(2026, 7, 24, 15, 0, 0),
                ScreenerPeriod::Week
            )
            .last()
            .unwrap()
            .trading_date,
            naive_date("2026-07-24")
        );
    }
}
