pub struct TrendResult {
    pub state: String,
    pub ma_short: Option<f64>,
    pub ma_long: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct QuantGridZoneInternal {
    pub zone: String,
    pub direction: String,
    pub lower: f64,
    pub upper: f64,
}

pub struct SignalDecision {
    pub output_state: String,
    pub current_trigger_zone: Option<String>,
}

pub fn calculate_ma(closes: &[f64], period: usize) -> Option<f64> {
    if period == 0 || closes.len() < period {
        return None;
    }
    let slice = &closes[closes.len() - period..];
    Some(slice.iter().sum::<f64>() / period as f64)
}

pub fn classify_trend(
    bars: &[crate::models::KlineBar],
    ma_short: usize,
    ma_long: usize,
) -> TrendResult {
    let closes: Vec<f64> = bars.iter().map(|bar| bar.close).collect();
    let short = calculate_ma(&closes, ma_short);
    let long = calculate_ma(&closes, ma_long);
    let latest = closes.last().copied();
    let state = match (short, long, latest) {
        (Some(s), Some(l), Some(c)) if s > l && c >= s => "bullish",
        (Some(s), Some(l), Some(c)) if s < l && c <= s => "bearish",
        (Some(_), Some(_), Some(_)) => "neutral",
        _ => "insufficient_data",
    }
    .to_string();

    TrendResult {
        state,
        ma_short: short,
        ma_long: long,
    }
}

pub fn generate_grid_zones(
    bars: &[crate::models::KlineBar],
    lookback_days: usize,
) -> Option<Vec<QuantGridZoneInternal>> {
    if bars.len() < lookback_days || lookback_days < 2 {
        return None;
    }

    let window = &bars[bars.len() - lookback_days..];
    let base = window.last()?.close;
    let high = window.iter().map(|bar| bar.high).fold(f64::MIN, f64::max);
    let low = window.iter().map(|bar| bar.low).fold(f64::MAX, f64::min);
    let range_step = (high - low) / 6.0;
    let moves: Vec<f64> = window
        .windows(2)
        .map(|pair| (pair[1].close - pair[0].close).abs())
        .collect();
    let avg_abs_move = moves.iter().sum::<f64>() / moves.len() as f64;
    let step = range_step.max(avg_abs_move).max(base * 0.003);

    Some(vec![
        QuantGridZoneInternal {
            zone: "buy_1".into(),
            direction: "buy_attention".into(),
            lower: base - step,
            upper: base,
        },
        QuantGridZoneInternal {
            zone: "buy_2".into(),
            direction: "buy_attention".into(),
            lower: base - 2.0 * step,
            upper: base - step,
        },
        QuantGridZoneInternal {
            zone: "sell_1".into(),
            direction: "sell_attention".into(),
            lower: base,
            upper: base + step,
        },
        QuantGridZoneInternal {
            zone: "sell_2".into(),
            direction: "sell_attention".into(),
            lower: base + step,
            upper: base + 2.0 * step,
        },
    ])
}

pub fn decide_signal(
    price: f64,
    trend_state: &str,
    zones: &[QuantGridZoneInternal],
) -> SignalDecision {
    if trend_state == "insufficient_data" {
        return SignalDecision {
            output_state: "watch".into(),
            current_trigger_zone: None,
        };
    }

    for zone in zones {
        let in_zone = match zone.zone.as_str() {
            "buy_1" | "buy_2" => price >= zone.lower && price < zone.upper,
            "sell_1" | "sell_2" => price > zone.lower && price <= zone.upper,
            _ => false,
        };

        if !in_zone {
            continue;
        }

        if zone.direction == "buy_attention" && trend_state != "bearish" {
            return SignalDecision {
                output_state: zone.direction.clone(),
                current_trigger_zone: Some(zone.zone.clone()),
            };
        }

        if zone.direction == "sell_attention" && trend_state != "bullish" {
            return SignalDecision {
                output_state: zone.direction.clone(),
                current_trigger_zone: Some(zone.zone.clone()),
            };
        }
    }

    SignalDecision {
        output_state: "watch".into(),
        current_trigger_zone: None,
    }
}

pub fn dedupe_key(code: &str, market: &str, direction: &str, trigger_zone: &str) -> String {
    format!("{market}:{code}:{direction}:{trigger_zone}")
}

pub fn completed_daily_bars(
    bars: &[crate::models::KlineBar],
    now: chrono::DateTime<chrono::FixedOffset>,
) -> Vec<crate::models::KlineBar> {
    let today = now.format("%Y-%m-%d").to_string();
    let market_closed = now.time() > chrono::NaiveTime::from_hms_opt(15, 0, 0).unwrap();
    bars.iter()
        .filter(|bar| market_closed || bar.date != today)
        .cloned()
        .collect()
}

pub fn china_market_now() -> chrono::DateTime<chrono::FixedOffset> {
    let offset = chrono::FixedOffset::east_opt(8 * 3600).unwrap();
    chrono::Utc::now().with_timezone(&offset)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TradingSession {
    Morning,
    Afternoon,
}

fn trading_session_for_time(time: chrono::NaiveTime) -> Option<TradingSession> {
    let morning = time >= chrono::NaiveTime::from_hms_opt(9, 30, 0).unwrap()
        && time <= chrono::NaiveTime::from_hms_opt(11, 30, 0).unwrap();
    if morning {
        return Some(TradingSession::Morning);
    }

    let afternoon = time >= chrono::NaiveTime::from_hms_opt(13, 0, 0).unwrap()
        && time <= chrono::NaiveTime::from_hms_opt(15, 0, 0).unwrap();
    if afternoon {
        return Some(TradingSession::Afternoon);
    }

    None
}

pub fn is_trading_time(now: chrono::DateTime<chrono::FixedOffset>) -> bool {
    use chrono::{Datelike, Weekday};

    if matches!(now.weekday(), Weekday::Sat | Weekday::Sun) {
        return false;
    }

    trading_session_for_time(now.time()).is_some()
}

pub fn next_refresh_at(
    now: chrono::DateTime<chrono::FixedOffset>,
    poll_interval_seconds: i64,
) -> Option<String> {
    if !is_trading_time(now) {
        return None;
    }

    Some(
        (now + chrono::Duration::seconds(poll_interval_seconds))
            .format("%Y-%m-%d %H:%M:%S")
            .to_string(),
    )
}

pub fn quote_timestamp_is_current_session(
    exchange_timestamp: i64,
    now: chrono::DateTime<chrono::FixedOffset>,
) -> bool {
    if !is_trading_time(now) {
        return false;
    }
    let Some(now_session) = trading_session_for_time(now.time()) else {
        return false;
    };

    let quote_time = chrono::DateTime::from_timestamp(exchange_timestamp, 0)
        .map(|dt| dt.with_timezone(now.offset()));

    match quote_time {
        Some(ts) => {
            ts.date_naive() == now.date_naive()
                && trading_session_for_time(ts.time()) == Some(now_session)
        }
        None => false,
    }
}

pub fn quote_status_allows_strict_signal(market_status: Option<&str>) -> bool {
    matches!(market_status.map(str::trim), Some("5") | Some("交易中"))
}

pub fn quote_is_strictly_realtime(
    quote: &crate::services::quote::RealtimeStockQuote,
    now: chrono::DateTime<chrono::FixedOffset>,
) -> bool {
    if let Some(ts) = quote.exchange_timestamp {
        return quote_timestamp_is_current_session(ts, now);
    }

    is_trading_time(now) && quote_status_allows_strict_signal(quote.market_status.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::KlineBar;
    use chrono::{Days, FixedOffset, NaiveDate, TimeZone};

    fn bars(closes: &[f64]) -> Vec<KlineBar> {
        let start = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();

        closes
            .iter()
            .enumerate()
            .map(|(index, close)| KlineBar {
                date: start
                    .checked_add_days(Days::new(index as u64))
                    .unwrap()
                    .format("%Y-%m-%d")
                    .to_string(),
                open: *close,
                close: *close,
                low: close - 1.0,
                high: close + 1.0,
                volume: 1_000.0,
                change_pct: 0.0,
            })
            .collect()
    }

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-9,
            "expected {actual} to be close to {expected}"
        );
    }

    fn cn_datetime(
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        minute: u32,
        second: u32,
    ) -> chrono::DateTime<FixedOffset> {
        FixedOffset::east_opt(8 * 60 * 60)
            .unwrap()
            .with_ymd_and_hms(year, month, day, hour, minute, second)
            .unwrap()
    }

    #[test]
    fn trading_time_rejects_weekend() {
        assert!(!is_trading_time(cn_datetime(2026, 7, 11, 10, 0, 0)));
    }

    #[test]
    fn trading_time_accepts_morning_session() {
        assert!(is_trading_time(cn_datetime(2026, 7, 7, 10, 0, 0)));
    }

    #[test]
    fn trading_time_rejects_seconds_after_session_boundaries() {
        assert!(is_trading_time(cn_datetime(2026, 7, 7, 11, 30, 0)));
        assert!(!is_trading_time(cn_datetime(2026, 7, 7, 11, 30, 59)));
        assert!(is_trading_time(cn_datetime(2026, 7, 7, 15, 0, 0)));
        assert!(!is_trading_time(cn_datetime(2026, 7, 7, 15, 0, 59)));
    }

    #[test]
    fn strict_quote_rejects_outside_session_time() {
        let now = cn_datetime(2026, 7, 7, 10, 0, 0);
        let timestamp = cn_datetime(2026, 7, 7, 8, 0, 0).timestamp();

        assert!(!quote_timestamp_is_current_session(timestamp, now));
    }

    #[test]
    fn strict_quote_rejects_wrong_date_timestamp() {
        let now = cn_datetime(2026, 7, 7, 10, 0, 0);
        let timestamp = cn_datetime(2026, 7, 6, 10, 0, 0).timestamp();

        assert!(!quote_timestamp_is_current_session(timestamp, now));
    }

    #[test]
    fn strict_quote_rejects_timestamp_when_now_outside_trading_time() {
        let now = cn_datetime(2026, 7, 7, 15, 30, 0);
        let timestamp = cn_datetime(2026, 7, 7, 14, 59, 0).timestamp();

        assert!(!quote_timestamp_is_current_session(timestamp, now));
    }

    #[test]
    fn strict_quote_rejects_morning_timestamp_during_afternoon_session() {
        let now = cn_datetime(2026, 7, 7, 13, 5, 0);
        let timestamp = cn_datetime(2026, 7, 7, 10, 30, 0).timestamp();

        assert!(!quote_timestamp_is_current_session(timestamp, now));
    }

    #[test]
    fn strict_quote_rejects_afternoon_timestamp_during_morning_session() {
        let now = cn_datetime(2026, 7, 7, 10, 30, 0);
        let timestamp = cn_datetime(2026, 7, 7, 13, 5, 0).timestamp();

        assert!(!quote_timestamp_is_current_session(timestamp, now));
    }

    #[test]
    fn strict_quote_accepts_timestamp_in_same_active_session() {
        let morning_now = cn_datetime(2026, 7, 7, 10, 30, 0);
        let morning_timestamp = cn_datetime(2026, 7, 7, 9, 30, 0).timestamp();
        let afternoon_now = cn_datetime(2026, 7, 7, 13, 5, 0);
        let afternoon_timestamp = cn_datetime(2026, 7, 7, 15, 0, 0).timestamp();

        assert!(quote_timestamp_is_current_session(
            morning_timestamp,
            morning_now
        ));
        assert!(quote_timestamp_is_current_session(
            afternoon_timestamp,
            afternoon_now
        ));
    }

    #[test]
    fn strict_quote_rejects_stale_timestamp_even_with_open_status() {
        let quote = crate::services::quote::RealtimeStockQuote {
            price: 10.0,
            exchange_timestamp: Some(cn_datetime(2026, 7, 6, 10, 0, 0).timestamp()),
            market_status: Some("5".into()),
            quote_fetched_at: "2026-07-07 10:00:00".into(),
        };

        assert!(!quote_is_strictly_realtime(
            &quote,
            cn_datetime(2026, 7, 7, 10, 0, 0)
        ));
    }

    #[test]
    fn strict_quote_rejects_open_status_without_timestamp_outside_trading_time() {
        let quote = crate::services::quote::RealtimeStockQuote {
            price: 10.0,
            exchange_timestamp: None,
            market_status: Some("5".into()),
            quote_fetched_at: "2026-07-07 15:30:00".into(),
        };

        assert!(!quote_is_strictly_realtime(
            &quote,
            cn_datetime(2026, 7, 7, 15, 30, 0)
        ));
        assert!(!quote_is_strictly_realtime(
            &quote,
            cn_datetime(2026, 7, 11, 10, 0, 0)
        ));
    }

    #[test]
    fn strict_quote_accepts_open_status_without_timestamp_during_trading_time() {
        let quote = crate::services::quote::RealtimeStockQuote {
            price: 10.0,
            exchange_timestamp: None,
            market_status: Some("5".into()),
            quote_fetched_at: "2026-07-07 10:00:00".into(),
        };

        assert!(quote_is_strictly_realtime(
            &quote,
            cn_datetime(2026, 7, 7, 10, 0, 0)
        ));
    }

    #[test]
    fn calculates_ma() {
        assert_eq!(calculate_ma(&[1.0, 2.0, 3.0, 4.0], 3), Some(3.0));
        assert_eq!(calculate_ma(&[1.0, 2.0], 3), None);
        assert_eq!(calculate_ma(&[1.0, 2.0], 0), None);
    }

    #[test]
    fn classifies_trend() {
        let bullish = classify_trend(&bars(&[10.0, 11.0, 12.0, 13.0, 14.0]), 2, 4);
        assert_eq!(bullish.state, "bullish");
        assert_close(bullish.ma_short.unwrap(), 13.5);
        assert_close(bullish.ma_long.unwrap(), 12.5);

        let bearish = classify_trend(&bars(&[14.0, 13.0, 12.0, 11.0, 10.0]), 2, 4);
        assert_eq!(bearish.state, "bearish");

        let neutral = classify_trend(&bars(&[10.0, 11.0, 10.0, 11.0]), 2, 4);
        assert_eq!(neutral.state, "neutral");

        let insufficient = classify_trend(&bars(&[10.0]), 2, 4);
        assert_eq!(insufficient.state, "insufficient_data");
        assert_eq!(insufficient.ma_short, None);
        assert_eq!(insufficient.ma_long, None);
    }

    #[test]
    fn generates_grid_zones() {
        let zones = generate_grid_zones(&bars(&[100.0, 102.0, 104.0, 106.0]), 4).unwrap();

        assert_eq!(zones.len(), 4);
        assert_eq!(zones[0].zone, "buy_1");
        assert_eq!(zones[0].direction, "buy_attention");
        assert_close(zones[0].lower, 104.0);
        assert_close(zones[0].upper, 106.0);
        assert_eq!(zones[1].zone, "buy_2");
        assert_eq!(zones[2].zone, "sell_1");
        assert_eq!(zones[3].zone, "sell_2");
    }

    #[test]
    fn decides_signal_from_grid_and_trend() {
        let zones = vec![
            QuantGridZoneInternal {
                zone: "buy_1".into(),
                direction: "buy_attention".into(),
                lower: 90.0,
                upper: 100.0,
            },
            QuantGridZoneInternal {
                zone: "sell_1".into(),
                direction: "sell_attention".into(),
                lower: 100.0,
                upper: 110.0,
            },
        ];

        let buy = decide_signal(95.0, "neutral", &zones);
        assert_eq!(buy.output_state, "buy_attention");
        assert_eq!(buy.current_trigger_zone, Some("buy_1".into()));

        let sell = decide_signal(105.0, "neutral", &zones);
        assert_eq!(sell.output_state, "sell_attention");
        assert_eq!(sell.current_trigger_zone, Some("sell_1".into()));

        let watch = decide_signal(120.0, "neutral", &zones);
        assert_eq!(watch.output_state, "watch");
        assert_eq!(watch.current_trigger_zone, None);
    }

    #[test]
    fn cooldown_key_includes_market() {
        assert_eq!(
            dedupe_key("00700", "HK", "buy", "buy_1"),
            "HK:00700:buy:buy_1"
        );
        assert_ne!(
            dedupe_key("00700", "HK", "buy", "buy_1"),
            dedupe_key("00700", "US", "buy", "buy_1")
        );
    }

    #[test]
    fn insufficient_data_does_not_create_grid() {
        assert!(generate_grid_zones(&bars(&[100.0, 101.0]), 3).is_none());
        assert!(generate_grid_zones(&bars(&[100.0, 101.0]), 1).is_none());
    }

    #[test]
    fn grid_boundaries_match_spec() {
        let zones = vec![
            QuantGridZoneInternal {
                zone: "buy_1".into(),
                direction: "buy_attention".into(),
                lower: 90.0,
                upper: 100.0,
            },
            QuantGridZoneInternal {
                zone: "sell_1".into(),
                direction: "sell_attention".into(),
                lower: 100.0,
                upper: 110.0,
            },
        ];

        assert_eq!(
            decide_signal(90.0, "neutral", &zones).current_trigger_zone,
            Some("buy_1".into())
        );
        assert_eq!(
            decide_signal(100.0, "neutral", &zones).output_state,
            "watch"
        );
        assert_eq!(
            decide_signal(110.0, "neutral", &zones).current_trigger_zone,
            Some("sell_1".into())
        );
    }

    #[test]
    fn trend_filter_suppresses_opposite_signals() {
        let zones = vec![
            QuantGridZoneInternal {
                zone: "buy_1".into(),
                direction: "buy_attention".into(),
                lower: 90.0,
                upper: 100.0,
            },
            QuantGridZoneInternal {
                zone: "sell_1".into(),
                direction: "sell_attention".into(),
                lower: 100.0,
                upper: 110.0,
            },
        ];

        assert_eq!(decide_signal(95.0, "bearish", &zones).output_state, "watch");
        assert_eq!(
            decide_signal(105.0, "bullish", &zones).output_state,
            "watch"
        );
    }

    #[test]
    fn insufficient_data_trend_suppresses_all_signals() {
        let zones = vec![
            QuantGridZoneInternal {
                zone: "buy_1".into(),
                direction: "buy_attention".into(),
                lower: 90.0,
                upper: 100.0,
            },
            QuantGridZoneInternal {
                zone: "sell_1".into(),
                direction: "sell_attention".into(),
                lower: 100.0,
                upper: 110.0,
            },
        ];

        let buy = decide_signal(95.0, "insufficient_data", &zones);
        assert_eq!(buy.output_state, "watch");
        assert_eq!(buy.current_trigger_zone, None);

        let sell = decide_signal(105.0, "insufficient_data", &zones);
        assert_eq!(sell.output_state, "watch");
        assert_eq!(sell.current_trigger_zone, None);
    }

    #[test]
    fn excludes_unfinished_current_day_bar() {
        let tz = FixedOffset::east_opt(8 * 60 * 60).unwrap();
        let now = tz.with_ymd_and_hms(2026, 7, 7, 10, 0, 0).unwrap();
        let exactly_close = tz.with_ymd_and_hms(2026, 7, 7, 15, 0, 0).unwrap();
        let mut input = bars(&[100.0, 101.0]);
        input[0].date = "2026-07-06".into();
        input[1].date = "2026-07-07".into();

        let completed = completed_daily_bars(&input, now);
        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0].date, "2026-07-06");

        let completed_at_close = completed_daily_bars(&input, exactly_close);
        assert_eq!(completed_at_close.len(), 1);
        assert_eq!(completed_at_close[0].date, "2026-07-06");
    }

    #[test]
    fn excluding_current_day_still_keeps_twenty_completed_bars() {
        let tz = FixedOffset::east_opt(8 * 60 * 60).unwrap();
        let now = tz.with_ymd_and_hms(2026, 7, 7, 10, 0, 0).unwrap();
        let mut input = bars(&[100.0; 21]);
        input[20].date = "2026-07-07".into();

        let completed = completed_daily_bars(&input, now);

        assert_eq!(completed.len(), 20);
        assert!(completed.iter().all(|bar| bar.date != "2026-07-07"));
    }
}
