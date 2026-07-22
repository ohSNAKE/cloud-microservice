#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, FixedOffset, NaiveDate, TimeZone};

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

    fn dividend(
        date: &str,
        amount: f64,
        adjustment: DividendAdjustment,
    ) -> NormalizedDividend {
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
        let mut bars = (0..19)
            .map(|index| {
                let mut item = bar(&format!("2026-{:02}-03", index % 6 + 1), 10.0);
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
        assert!(!is_central_soe("某地方国资委控股的中国移动通信集团", &registry));
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
        assert!(lower_boll_band(
            &bars,
            china_datetime(2026, 7, 21, 15, 0, 0),
            ScreenerPeriod::Day
        )
        .unwrap()
            < 10.0);
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
            lower_boll_band(&bars, china_datetime(2026, 7, 24, 14, 59, 59), ScreenerPeriod::Week),
            Some(10.0)
        );
    }

    #[test]
    fn friday_bar_is_included_at_close_and_future_weekly_bars_are_excluded() {
        let bars = weekly_bars_including("2026-07-24", 8.0);
        assert!(lower_boll_band(
            &bars,
            china_datetime(2026, 7, 24, 15, 0, 0),
            ScreenerPeriod::Week
        )
        .unwrap()
            < 10.0);
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
