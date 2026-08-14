use std::collections::{BTreeMap, HashMap};

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

#[derive(Clone, Copy)]
struct IntradayTBucket {
    label: &'static str,
    start: &'static str,
    end: &'static str,
}

const INTRADAY_T_BUCKETS: [IntradayTBucket; 6] = [
    IntradayTBucket {
        label: "09:30-09:45",
        start: "09:30",
        end: "09:45",
    },
    IntradayTBucket {
        label: "09:50-10:30",
        start: "09:50",
        end: "10:30",
    },
    IntradayTBucket {
        label: "10:35-11:30",
        start: "10:35",
        end: "11:30",
    },
    IntradayTBucket {
        label: "13:00-13:30",
        start: "13:00",
        end: "13:30",
    },
    IntradayTBucket {
        label: "13:35-14:30",
        start: "13:35",
        end: "14:30",
    },
    IntradayTBucket {
        label: "14:35-15:00",
        start: "14:35",
        end: "15:00",
    },
];

const INTRADAY_T_EXPECTED_5M_BARS: usize = 50;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct IntradayTWindowStats {
    pub valid_days: usize,
    pub high_frequency_windows: Vec<String>,
    pub low_frequency_windows: Vec<String>,
}

pub struct IntradayTDecision {
    pub output_state: String,
    pub current_trigger_zone: Option<String>,
    pub reason: Option<String>,
}

pub const INTRADAY_T_SELL_CONFIRMATION_TIME: &str = "09:35:00";
pub const INTRADAY_T_MIN_HIGH_RETREAT: f64 = 0.005;
pub const INTRADAY_T_MIN_LOW_REBOUND: f64 = 0.006;

pub struct IntradayTSignalInput<'a> {
    pub now: chrono::DateTime<chrono::FixedOffset>,
    pub position: f64,
    pub stats: &'a IntradayTWindowStats,
    pub sell_threshold: f64,
    pub buyback_threshold: f64,
    pub current_price: f64,
    pub minute_bars: &'a [crate::models::KlineBar],
    pub sold_at: Option<&'a str>,
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

fn parse_intraday_time(date_time: &str) -> Option<chrono::NaiveTime> {
    let time = date_time.split_whitespace().nth(1)?;
    chrono::NaiveTime::parse_from_str(time, "%H:%M")
        .or_else(|_| chrono::NaiveTime::parse_from_str(time, "%H:%M:%S"))
        .ok()
}

fn date_prefix(date_time: &str) -> Option<&str> {
    date_time.split_whitespace().next()
}

pub fn intraday_t_bucket(date_time: &str) -> Option<String> {
    let time = parse_intraday_time(date_time)?;
    for bucket in INTRADAY_T_BUCKETS {
        let start = chrono::NaiveTime::parse_from_str(bucket.start, "%H:%M").unwrap();
        let end = chrono::NaiveTime::parse_from_str(bucket.end, "%H:%M").unwrap();
        if time >= start && time <= end {
            return Some(bucket.label.to_string());
        }
    }
    None
}

pub fn group_intraday_bars_by_day(
    bars: &[crate::models::KlineBar],
) -> Vec<(String, Vec<crate::models::KlineBar>)> {
    let mut grouped: BTreeMap<String, Vec<crate::models::KlineBar>> = BTreeMap::new();
    for bar in bars {
        if let Some(date) = date_prefix(&bar.date) {
            grouped
                .entry(date.to_string())
                .or_default()
                .push(bar.clone());
        }
    }
    grouped.into_iter().collect()
}

pub fn analyze_intraday_t_windows(
    bars: &[crate::models::KlineBar],
    current_date: &str,
    lookback_days: usize,
    high_min_count: usize,
    low_min_count: usize,
) -> IntradayTWindowStats {
    let mut valid_completed_days: Vec<_> = group_intraday_bars_by_day(bars)
        .into_iter()
        .filter(|(date, _)| date != current_date)
        .filter_map(|(_, day_bars)| {
            let has_morning = day_bars.iter().any(|bar| {
                parse_intraday_time(&bar.date).and_then(trading_session_for_time)
                    == Some(TradingSession::Morning)
            });
            let has_afternoon = day_bars.iter().any(|bar| {
                parse_intraday_time(&bar.date).and_then(trading_session_for_time)
                    == Some(TradingSession::Afternoon)
            });
            (day_bars.len() >= INTRADAY_T_EXPECTED_5M_BARS && has_morning && has_afternoon)
                .then_some(day_bars)
        })
        .collect();

    if valid_completed_days.len() > lookback_days {
        valid_completed_days =
            valid_completed_days.split_off(valid_completed_days.len() - lookback_days);
    }

    let mut high_counts: HashMap<String, usize> = HashMap::new();
    let mut low_counts: HashMap<String, usize> = HashMap::new();

    for day_bars in &valid_completed_days {
        if let Some(high_bar) = day_bars
            .iter()
            .max_by(|left, right| left.high.total_cmp(&right.high))
        {
            if let Some(bucket) = intraday_t_bucket(&high_bar.date) {
                *high_counts.entry(bucket).or_default() += 1;
            }
        }

        if let Some(low_bar) = day_bars
            .iter()
            .min_by(|left, right| left.low.total_cmp(&right.low))
        {
            if let Some(bucket) = intraday_t_bucket(&low_bar.date) {
                *low_counts.entry(bucket).or_default() += 1;
            }
        }
    }

    IntradayTWindowStats {
        valid_days: valid_completed_days.len(),
        high_frequency_windows: INTRADAY_T_BUCKETS
            .iter()
            .filter(|bucket| {
                high_counts.get(bucket.label).copied().unwrap_or_default() >= high_min_count
            })
            .map(|bucket| bucket.label.to_string())
            .collect(),
        low_frequency_windows: INTRADAY_T_BUCKETS
            .iter()
            .filter(|bucket| {
                low_counts.get(bucket.label).copied().unwrap_or_default() >= low_min_count
            })
            .map(|bucket| bucket.label.to_string())
            .collect(),
    }
}

pub fn intraday_t_day_position(
    bars: &[crate::models::KlineBar],
    current_date: &str,
    current_price: f64,
    min_range: f64,
) -> Option<f64> {
    let mut high = current_price;
    let mut low = current_price;

    for bar in bars
        .iter()
        .filter(|bar| date_prefix(&bar.date) == Some(current_date))
    {
        high = high.max(bar.high);
        low = low.min(bar.low);
    }

    let range = high - low;
    if range <= min_range {
        return None;
    }

    Some((current_price - low) / range)
}

pub fn decide_intraday_t_signal(
    now: chrono::DateTime<chrono::FixedOffset>,
    position: f64,
    stats: &IntradayTWindowStats,
    sell_threshold: f64,
    buyback_threshold: f64,
) -> IntradayTDecision {
    let Some(bucket) = intraday_t_bucket(&now.format("%Y-%m-%d %H:%M:%S").to_string()) else {
        return IntradayTDecision {
            output_state: "watch".into(),
            current_trigger_zone: None,
            reason: Some("未进入高发时段或日内位置未达阈值".into()),
        };
    };

    let sell_matches = stats.high_frequency_windows.contains(&bucket) && position >= sell_threshold;
    let buyback_matches =
        stats.low_frequency_windows.contains(&bucket) && position <= buyback_threshold;

    if sell_matches && buyback_matches {
        return IntradayTDecision {
            output_state: "watch".into(),
            current_trigger_zone: None,
            reason: Some("卖T与买回条件冲突，防御性观望".into()),
        };
    }

    if sell_matches {
        return intraday_t_decision(
            "sell_t_attention",
            bucket.clone(),
            format!("卖T关注 · 历史高点高发时段 {bucket}"),
        );
    }

    if buyback_matches {
        return intraday_t_decision(
            "buyback_attention",
            bucket.clone(),
            format!("买回关注 · 历史低点高发时段 {bucket}"),
        );
    }

    IntradayTDecision {
        output_state: "watch".into(),
        current_trigger_zone: None,
        reason: Some("未进入高发时段或日内位置未达阈值".into()),
    }
}

fn intraday_t_watch_decision(now: chrono::DateTime<chrono::FixedOffset>) -> IntradayTDecision {
    if is_intraday_t_late_window(now.time()) {
        return IntradayTDecision {
            output_state: "watch".into(),
            current_trigger_zone: None,
            reason: Some("尾盘时段仅观察，不触发日内T信号".into()),
        };
    }

    IntradayTDecision {
        output_state: "watch".into(),
        current_trigger_zone: None,
        reason: Some("缺少日内T确认数据，暂不触发".into()),
    }
}

fn intraday_t_decision(output_state: &str, bucket: String, reason: String) -> IntradayTDecision {
    IntradayTDecision {
        output_state: output_state.into(),
        current_trigger_zone: Some(bucket),
        reason: Some(reason),
    }
}

fn is_intraday_t_late_window(time: chrono::NaiveTime) -> bool {
    let start = chrono::NaiveTime::from_hms_opt(14, 35, 0).unwrap();
    let end = chrono::NaiveTime::from_hms_opt(15, 0, 0).unwrap();
    time >= start && time <= end
}

fn parse_intraday_timestamp(
    date_time: &str,
    offset: chrono::FixedOffset,
) -> Option<chrono::DateTime<chrono::FixedOffset>> {
    let naive = chrono::NaiveDateTime::parse_from_str(date_time, "%Y-%m-%d %H:%M:%S")
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(date_time, "%Y-%m-%d %H:%M"))
        .ok()?;
    chrono::TimeZone::from_local_datetime(&offset, &naive).single()
}

pub fn completed_intraday_bars(
    bars: &[crate::models::KlineBar],
    now: chrono::DateTime<chrono::FixedOffset>,
) -> Vec<crate::models::KlineBar> {
    let mut completed: Vec<_> = bars
        .iter()
        .filter_map(|bar| {
            let close_at = parse_intraday_timestamp(&bar.date, *now.offset())?;
            (close_at.date_naive() == now.date_naive() && close_at < now)
                .then(|| (close_at, bar.clone()))
        })
        .collect();
    completed.sort_by_key(|(close_at, _)| *close_at);
    completed.into_iter().map(|(_, bar)| bar).collect()
}

pub fn decide_guarded_intraday_t_signal(input: IntradayTSignalInput<'_>) -> IntradayTDecision {
    let Some(bucket) = intraday_t_bucket(&input.now.format("%Y-%m-%d %H:%M:%S").to_string()) else {
        return intraday_t_watch_decision(input.now);
    };

    if is_intraday_t_late_window(input.now.time()) {
        return intraday_t_watch_decision(input.now);
    }

    let sell_matches = input.stats.high_frequency_windows.contains(&bucket)
        && input.position >= input.sell_threshold;
    let opening_end = chrono::NaiveTime::from_hms_opt(9, 34, 59).unwrap();
    if sell_matches && input.now.time() <= opening_end {
        return intraday_t_decision("sell_t_watch", bucket, "早盘冲高预警，等待回落确认".into());
    }

    let completed_bars = completed_intraday_bars(input.minute_bars, input.now);
    let day_high = completed_bars
        .iter()
        .map(|bar| bar.high)
        .fold(input.current_price, f64::max);
    let day_low = completed_bars
        .iter()
        .map(|bar| bar.low)
        .fold(input.current_price, f64::min);
    let confirmation_start =
        chrono::NaiveTime::parse_from_str(INTRADAY_T_SELL_CONFIRMATION_TIME, "%H:%M:%S").unwrap();

    let sell_confirmed = sell_matches
        && input.now.time() >= confirmation_start
        && input.current_price <= day_high * (1.0 - INTRADAY_T_MIN_HIGH_RETREAT);

    let buyback_matches = input.stats.low_frequency_windows.contains(&bucket)
        && input.position <= input.buyback_threshold
        && input.current_price >= day_low * (1.0 + INTRADAY_T_MIN_LOW_REBOUND);
    let buyback_confirmed = buyback_matches
        && input
            .sold_at
            .and_then(|sold_at| parse_intraday_timestamp(sold_at, *input.now.offset()))
            .filter(|sold_at| sold_at.date_naive() == input.now.date_naive())
            .and_then(|sold_at| {
                let (low_index, low_bar) =
                    completed_bars.iter().enumerate().min_by(|left, right| {
                        left.1
                            .low
                            .total_cmp(&right.1.low)
                            .then_with(|| left.0.cmp(&right.0))
                    })?;
                let low_at = parse_intraday_timestamp(&low_bar.date, *input.now.offset())?;
                let confirmations: Vec<_> =
                    completed_bars.iter().skip(low_index + 1).take(2).collect();
                (sold_at < low_at
                    && confirmations.len() == 2
                    && confirmations[1].low >= confirmations[0].low)
                    .then_some(())
            })
            .is_some();

    if sell_confirmed && buyback_confirmed {
        return intraday_t_watch_decision(input.now);
    }

    if sell_confirmed {
        return intraday_t_decision(
            "sell_t_attention",
            bucket.clone(),
            format!("卖T确认 · 历史高点高发时段 {bucket}"),
        );
    }

    if buyback_confirmed {
        return intraday_t_decision(
            "buyback_attention",
            bucket.clone(),
            format!("买回确认 · 历史低点高发时段 {bucket}"),
        );
    }

    intraday_t_watch_decision(input.now)
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

pub fn quote_is_intraday_t_realtime(
    quote: &crate::services::quote::RealtimeStockQuote,
    now: chrono::DateTime<chrono::FixedOffset>,
) -> bool {
    let Some(exchange_timestamp) = quote.exchange_timestamp else {
        return false;
    };
    let Some(quote_time) = chrono::DateTime::from_timestamp(exchange_timestamp, 0)
        .map(|timestamp| timestamp.with_timezone(now.offset()))
    else {
        return false;
    };

    quote_timestamp_is_current_session(exchange_timestamp, now)
        && now.signed_duration_since(quote_time) >= chrono::Duration::zero()
        && now.signed_duration_since(quote_time) <= chrono::Duration::seconds(120)
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

    fn minute_bar(date: &str, open: f64, high: f64, low: f64) -> KlineBar {
        KlineBar {
            date: date.to_string(),
            open,
            close: open,
            low,
            high,
            volume: 1_000.0,
            change_pct: 0.0,
        }
    }

    fn intraday_t_valid_day(date: &str, high_time: &str, low_time: &str) -> Vec<KlineBar> {
        let times = [
            "09:30", "09:35", "09:40", "09:45", "09:50", "09:55", "10:00", "10:05", "10:10",
            "10:15", "10:20", "10:25", "10:30", "10:35", "10:40", "10:45", "10:50", "10:55",
            "11:00", "11:05", "11:10", "11:15", "11:20", "11:25", "11:30", "13:00", "13:05",
            "13:10", "13:15", "13:20", "13:25", "13:30", "13:35", "13:40", "13:45", "13:50",
            "13:55", "14:00", "14:05", "14:10", "14:15", "14:20", "14:25", "14:30", "14:35",
            "14:40", "14:45", "14:50", "14:55", "15:00",
        ];

        times
            .iter()
            .enumerate()
            .map(|(index, time)| {
                let high = if *time == high_time {
                    100.0
                } else {
                    20.0 + index as f64 * 0.01
                };
                let low = if *time == low_time {
                    1.0
                } else {
                    10.0 + index as f64 * 0.01
                };
                minute_bar(&format!("{date} {time}"), 15.0, high, low)
            })
            .collect()
    }

    fn intraday_t_history_with_repeated_highs_and_lows() -> Vec<KlineBar> {
        let mut bars = Vec::new();
        for day in 1..=10 {
            let date = format!("2026-07-{day:02}");
            let high_time = match day {
                1..=5 => "09:35",
                6 => "10:00",
                7 => "10:35",
                8 => "13:15",
                9 => "14:00",
                _ => "14:40",
            };
            let low_time = match day {
                1..=4 => "13:40",
                5 => "09:35",
                6 => "10:00",
                7 => "10:35",
                8 => "13:15",
                9 => "14:40",
                _ => "15:00",
            };
            bars.extend(intraday_t_valid_day(&date, high_time, low_time));
        }
        bars
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
    fn intraday_t_assigns_time_buckets() {
        assert_eq!(
            intraday_t_bucket("2026-07-07 09:35").as_deref(),
            Some("09:30-09:45")
        );
        assert_eq!(
            intraday_t_bucket("2026-07-07 10:00").as_deref(),
            Some("09:50-10:30")
        );
        assert_eq!(
            intraday_t_bucket("2026-07-07 10:35").as_deref(),
            Some("10:35-11:30")
        );
        assert_eq!(
            intraday_t_bucket("2026-07-07 13:15:00").as_deref(),
            Some("13:00-13:30")
        );
        assert_eq!(
            intraday_t_bucket("2026-07-07 14:00").as_deref(),
            Some("13:35-14:30")
        );
        assert_eq!(
            intraday_t_bucket("2026-07-07 14:50").as_deref(),
            Some("14:35-15:00")
        );
        assert_eq!(intraday_t_bucket("2026-07-07 11:45"), None);
    }

    #[test]
    fn intraday_t_groups_by_date_prefix() {
        let bars = vec![
            minute_bar("2026-07-01 09:35", 10.0, 11.0, 9.0),
            minute_bar("2026-07-01 10:00:00", 10.0, 10.5, 8.5),
            minute_bar("2026-07-02 09:35", 10.0, 11.0, 9.0),
        ];

        let grouped = group_intraday_bars_by_day(&bars);

        assert_eq!(grouped.len(), 2);
        assert_eq!(grouped[0].0, "2026-07-01");
        assert_eq!(grouped[0].1.len(), 2);
        assert_eq!(grouped[1].0, "2026-07-02");
    }

    #[test]
    fn intraday_t_detects_high_and_low_frequency_windows() {
        let bars = intraday_t_history_with_repeated_highs_and_lows();

        let stats = analyze_intraday_t_windows(&bars, "2026-07-11", 22, 4, 4);

        assert_eq!(stats.valid_days, 10);
        assert_eq!(stats.high_frequency_windows, vec!["09:30-09:45"]);
        assert_eq!(stats.low_frequency_windows, vec!["13:35-14:30"]);
    }

    #[test]
    fn intraday_t_excludes_current_day_from_frequency_stats() {
        let mut bars = intraday_t_history_with_repeated_highs_and_lows();
        bars.extend(intraday_t_valid_day("2026-07-11", "09:35", "13:40"));

        let stats = analyze_intraday_t_windows(&bars, "2026-07-11", 22, 6, 5);

        assert_eq!(stats.valid_days, 10);
        assert!(stats.high_frequency_windows.is_empty());
        assert!(stats.low_frequency_windows.is_empty());
    }

    #[test]
    fn intraday_t_partial_historical_day_does_not_count() {
        let mut bars = intraday_t_history_with_repeated_highs_and_lows();
        bars.extend([
            minute_bar("2026-07-11 09:35", 10.0, 200.0, 9.0),
            minute_bar("2026-07-11 13:40", 10.0, 11.0, 0.5),
        ]);

        let stats = analyze_intraday_t_windows(&bars, "2026-07-12", 22, 6, 5);

        assert_eq!(stats.valid_days, 10);
        assert!(stats.high_frequency_windows.is_empty());
        assert!(stats.low_frequency_windows.is_empty());
    }

    #[test]
    fn intraday_t_rejects_a_36_bar_cross_session_historical_day() {
        let mut bars = intraday_t_valid_day("2026-07-11", "09:35", "13:40");
        bars.truncate(36);

        let stats = analyze_intraday_t_windows(&bars, "2026-07-12", 22, 1, 1);

        assert_eq!(stats.valid_days, 0);
        assert!(stats.high_frequency_windows.is_empty());
        assert!(stats.low_frequency_windows.is_empty());
    }

    #[test]
    fn intraday_t_lookback_ignores_recent_invalid_days_before_trimming() {
        let mut bars = Vec::new();
        bars.extend(intraday_t_valid_day("2026-07-01", "09:35", "13:40"));
        bars.extend(intraday_t_valid_day("2026-07-02", "09:35", "13:40"));
        bars.extend([
            minute_bar("2026-07-03 09:35", 10.0, 200.0, 9.0),
            minute_bar("2026-07-03 13:40", 10.0, 11.0, 0.5),
        ]);

        let stats = analyze_intraday_t_windows(&bars, "2026-07-04", 2, 2, 2);

        assert_eq!(stats.valid_days, 2);
        assert_eq!(stats.high_frequency_windows, vec!["09:30-09:45"]);
        assert_eq!(stats.low_frequency_windows, vec!["13:35-14:30"]);
    }

    #[test]
    fn intraday_t_multiple_frequency_windows_follow_bucket_order() {
        let mut bars = Vec::new();
        bars.extend(intraday_t_valid_day("2026-07-01", "13:15", "14:40"));
        bars.extend(intraday_t_valid_day("2026-07-02", "09:35", "10:00"));
        bars.extend(intraday_t_valid_day("2026-07-03", "13:15", "14:40"));
        bars.extend(intraday_t_valid_day("2026-07-04", "09:35", "10:00"));

        let stats = analyze_intraday_t_windows(&bars, "2026-07-05", 22, 2, 2);

        assert_eq!(
            stats.high_frequency_windows,
            vec!["09:30-09:45", "13:00-13:30"]
        );
        assert_eq!(
            stats.low_frequency_windows,
            vec!["09:50-10:30", "14:35-15:00"]
        );
    }

    #[test]
    fn intraday_t_calculates_current_day_position() {
        let bars = vec![minute_bar("2026-07-07 09:35", 10.0, 12.0, 8.0)];

        let position = intraday_t_day_position(&bars, "2026-07-07", 11.0, 0.0001).unwrap();

        assert_close(position, 0.75);
    }

    #[test]
    fn intraday_t_position_includes_current_price_in_range() {
        let bars = vec![minute_bar("2026-07-07 09:35", 10.0, 12.0, 8.0)];

        let position = intraday_t_day_position(&bars, "2026-07-07", 14.0, 0.0001).unwrap();

        assert_close(position, 1.0);
    }

    #[test]
    fn intraday_t_rejects_narrow_current_day_range() {
        let bars = vec![minute_bar("2026-07-07 09:35", 10.0, 10.00001, 10.0)];

        assert!(intraday_t_day_position(&bars, "2026-07-07", 10.0, 0.0001).is_none());
    }

    #[test]
    fn intraday_t_opening_watch_is_limited_to_the_first_five_minutes() {
        let stats = IntradayTWindowStats {
            valid_days: 22,
            high_frequency_windows: vec!["09:30-09:45".into()],
            low_frequency_windows: vec![],
        };
        let bars = vec![minute_bar("2026-07-07 09:30", 95.0, 100.0, 90.0)];

        let opening = decide_guarded_intraday_t_signal(IntradayTSignalInput {
            now: cn_datetime(2026, 7, 7, 9, 30, 0),
            position: 0.76,
            stats: &stats,
            sell_threshold: 0.70,
            buyback_threshold: 0.30,
            current_price: 99.0,
            minute_bars: &bars,
            sold_at: None,
        });
        assert_eq!(opening.output_state, "sell_t_watch");
        assert_ne!(opening.output_state, "sell_t_attention");

        let last_opening_second = decide_guarded_intraday_t_signal(IntradayTSignalInput {
            now: cn_datetime(2026, 7, 7, 9, 34, 59),
            ..IntradayTSignalInput {
                now: cn_datetime(2026, 7, 7, 9, 30, 0),
                position: 0.76,
                stats: &stats,
                sell_threshold: 0.70,
                buyback_threshold: 0.30,
                current_price: 99.0,
                minute_bars: &bars,
                sold_at: None,
            }
        });
        assert_eq!(last_opening_second.output_state, "sell_t_watch");

        let after_opening = decide_guarded_intraday_t_signal(IntradayTSignalInput {
            now: cn_datetime(2026, 7, 7, 9, 35, 0),
            position: 0.76,
            stats: &stats,
            sell_threshold: 0.70,
            buyback_threshold: 0.30,
            current_price: 99.7,
            minute_bars: &bars,
            sold_at: None,
        });
        assert_eq!(after_opening.output_state, "watch");
    }

    #[test]
    fn intraday_t_confirms_sell_only_after_0935_and_a_half_percent_retreat() {
        let stats = IntradayTWindowStats {
            valid_days: 22,
            high_frequency_windows: vec!["09:30-09:45".into()],
            low_frequency_windows: vec![],
        };
        let bars = vec![minute_bar("2026-07-07 09:30", 95.0, 100.0, 90.0)];

        let decision = decide_guarded_intraday_t_signal(IntradayTSignalInput {
            now: cn_datetime(2026, 7, 7, 9, 35, 0),
            position: 0.76,
            stats: &stats,
            sell_threshold: 0.70,
            buyback_threshold: 0.30,
            current_price: 99.5,
            minute_bars: &bars,
            sold_at: None,
        });

        assert_eq!(decision.output_state, "sell_t_attention");
        assert_eq!(
            decision.current_trigger_zone.as_deref(),
            Some("09:30-09:45")
        );
    }

    #[test]
    fn intraday_t_buyback_requires_an_explicit_earlier_sell_and_two_completed_reversal_bars() {
        let stats = IntradayTWindowStats {
            valid_days: 22,
            high_frequency_windows: vec![],
            low_frequency_windows: vec!["09:30-09:45".into()],
        };
        let bars = vec![
            minute_bar("2026-07-07 09:30", 91.0, 92.0, 90.0),
            minute_bar("2026-07-07 09:35", 91.0, 92.0, 90.0),
            minute_bar("2026-07-07 09:40", 91.0, 92.0, 90.2),
        ];

        let missing_sell = decide_guarded_intraday_t_signal(IntradayTSignalInput {
            now: cn_datetime(2026, 7, 7, 9, 41, 0),
            position: 0.24,
            stats: &stats,
            sell_threshold: 0.70,
            buyback_threshold: 0.30,
            current_price: 90.54,
            minute_bars: &bars,
            sold_at: None,
        });
        assert_eq!(missing_sell.output_state, "watch");

        let forming_second_bar = decide_guarded_intraday_t_signal(IntradayTSignalInput {
            now: cn_datetime(2026, 7, 7, 9, 40, 0),
            position: 0.24,
            stats: &stats,
            sell_threshold: 0.70,
            buyback_threshold: 0.30,
            current_price: 90.54,
            minute_bars: &bars,
            sold_at: Some("2026-07-07 09:29:00"),
        });
        assert_eq!(forming_second_bar.output_state, "watch");

        let decision = decide_guarded_intraday_t_signal(IntradayTSignalInput {
            now: cn_datetime(2026, 7, 7, 9, 41, 0),
            position: 0.24,
            stats: &stats,
            sell_threshold: 0.70,
            buyback_threshold: 0.30,
            current_price: 90.54,
            minute_bars: &bars,
            sold_at: Some("2026-07-07 09:29:00"),
        });

        assert_eq!(decision.output_state, "buyback_attention");
        assert_eq!(
            decision.current_trigger_zone.as_deref(),
            Some("09:30-09:45")
        );
    }

    #[test]
    fn intraday_t_buyback_requires_sell_to_precede_the_day_low() {
        let stats = IntradayTWindowStats {
            valid_days: 22,
            high_frequency_windows: vec![],
            low_frequency_windows: vec!["09:30-09:45".into()],
        };
        let bars = vec![
            minute_bar("2026-07-07 09:30", 91.0, 92.0, 90.0),
            minute_bar("2026-07-07 09:35", 91.0, 92.0, 90.0),
            minute_bar("2026-07-07 09:40", 91.0, 92.0, 90.2),
        ];

        let decision = decide_guarded_intraday_t_signal(IntradayTSignalInput {
            now: cn_datetime(2026, 7, 7, 9, 41, 0),
            position: 0.24,
            stats: &stats,
            sell_threshold: 0.70,
            buyback_threshold: 0.30,
            current_price: 90.54,
            minute_bars: &bars,
            sold_at: Some("2026-07-07 09:30:00"),
        });

        assert_eq!(decision.output_state, "watch");
    }

    #[test]
    fn intraday_t_buyback_rejects_a_lower_second_reversal_low() {
        let stats = IntradayTWindowStats {
            valid_days: 22,
            high_frequency_windows: vec![],
            low_frequency_windows: vec!["09:30-09:45".into()],
        };
        let bars = vec![
            minute_bar("2026-07-07 09:30", 91.0, 92.0, 90.0),
            minute_bar("2026-07-07 09:35", 91.0, 92.0, 90.3),
            minute_bar("2026-07-07 09:40", 91.0, 92.0, 90.2),
        ];

        let decision = decide_guarded_intraday_t_signal(IntradayTSignalInput {
            now: cn_datetime(2026, 7, 7, 9, 41, 0),
            position: 0.24,
            stats: &stats,
            sell_threshold: 0.70,
            buyback_threshold: 0.30,
            current_price: 90.54,
            minute_bars: &bars,
            sold_at: Some("2026-07-07 09:29:00"),
        });

        assert_eq!(decision.output_state, "watch");
    }

    #[test]
    fn intraday_t_overlap_between_confirmed_sell_and_buyback_returns_watch() {
        let stats = IntradayTWindowStats {
            valid_days: 22,
            high_frequency_windows: vec!["09:30-09:45".into()],
            low_frequency_windows: vec!["09:30-09:45".into()],
        };
        let bars = vec![
            minute_bar("2026-07-07 09:30", 91.0, 100.0, 90.0),
            minute_bar("2026-07-07 09:35", 91.0, 92.0, 90.0),
            minute_bar("2026-07-07 09:40", 91.0, 92.0, 90.2),
        ];

        let decision = decide_guarded_intraday_t_signal(IntradayTSignalInput {
            now: cn_datetime(2026, 7, 7, 9, 41, 0),
            position: 0.50,
            stats: &stats,
            sell_threshold: 0.50,
            buyback_threshold: 0.50,
            current_price: 99.5,
            minute_bars: &bars,
            sold_at: Some("2026-07-07 09:29:00"),
        });

        assert_eq!(decision.output_state, "watch");
    }

    #[test]
    fn intraday_t_uses_close_timestamps_and_excludes_the_bar_closing_now() {
        let bars = vec![
            minute_bar("2026-07-07 09:30", 91.0, 92.0, 90.0),
            minute_bar("2026-07-07 09:35", 91.0, 92.0, 90.0),
        ];

        assert_eq!(
            completed_intraday_bars(&bars, cn_datetime(2026, 7, 7, 9, 35, 0)).len(),
            1
        );
        assert_eq!(
            completed_intraday_bars(&bars, cn_datetime(2026, 7, 7, 9, 36, 0)).len(),
            2
        );
    }

    #[test]
    fn intraday_t_late_session_suppresses_all_intraday_t_states() {
        let stats = IntradayTWindowStats {
            valid_days: 22,
            high_frequency_windows: vec!["14:35-15:00".into()],
            low_frequency_windows: vec!["14:35-15:00".into()],
        };
        let bars = vec![
            minute_bar("2026-07-07 14:40", 91.0, 100.0, 90.0),
            minute_bar("2026-07-07 14:45", 91.0, 92.0, 90.0),
            minute_bar("2026-07-07 14:50", 91.0, 92.0, 90.2),
        ];

        for now in [
            cn_datetime(2026, 7, 7, 14, 35, 0),
            cn_datetime(2026, 7, 7, 15, 0, 0),
        ] {
            let decision = decide_guarded_intraday_t_signal(IntradayTSignalInput {
                now,
                position: 0.76,
                stats: &stats,
                sell_threshold: 0.70,
                buyback_threshold: 0.80,
                current_price: 99.0,
                minute_bars: &bars,
                sold_at: Some("2026-07-07 14:30:00"),
            });
            assert_eq!(decision.output_state, "watch");
        }
    }

    #[test]
    fn intraday_t_late_window_suppresses_an_otherwise_confirmed_sell() {
        let stats = IntradayTWindowStats {
            valid_days: 22,
            high_frequency_windows: vec!["14:35-15:00".into()],
            low_frequency_windows: vec![],
        };
        let bars = vec![minute_bar("2026-07-07 14:40", 95.0, 100.0, 90.0)];

        let decision = decide_guarded_intraday_t_signal(IntradayTSignalInput {
            now: cn_datetime(2026, 7, 7, 14, 50, 0),
            position: 0.76,
            stats: &stats,
            sell_threshold: 0.70,
            buyback_threshold: 0.30,
            current_price: 99.5,
            minute_bars: &bars,
            sold_at: None,
        });

        assert_eq!(decision.output_state, "watch");
    }

    #[test]
    fn intraday_t_late_window_suppresses_an_otherwise_confirmed_buyback() {
        let stats = IntradayTWindowStats {
            valid_days: 22,
            high_frequency_windows: vec![],
            low_frequency_windows: vec!["14:35-15:00".into()],
        };
        let bars = vec![
            minute_bar("2026-07-07 14:35", 91.0, 92.0, 90.0),
            minute_bar("2026-07-07 14:40", 91.0, 92.0, 90.0),
            minute_bar("2026-07-07 14:45", 91.0, 92.0, 90.2),
        ];

        let decision = decide_guarded_intraday_t_signal(IntradayTSignalInput {
            now: cn_datetime(2026, 7, 7, 14, 50, 0),
            position: 0.24,
            stats: &stats,
            sell_threshold: 0.70,
            buyback_threshold: 0.30,
            current_price: 90.54,
            minute_bars: &bars,
            sold_at: Some("2026-07-07 14:30:00"),
        });

        assert_eq!(decision.output_state, "watch");
    }

    #[test]
    fn legacy_intraday_t_emits_sell_and_buyback_from_time_plus_position() {
        let stats = IntradayTWindowStats {
            valid_days: 22,
            high_frequency_windows: vec!["09:30-09:45".into()],
            low_frequency_windows: vec!["13:35-14:30".into()],
        };

        let sell =
            decide_intraday_t_signal(cn_datetime(2026, 7, 7, 9, 35, 0), 0.76, &stats, 0.70, 0.30);
        assert_eq!(sell.output_state, "sell_t_attention");
        assert_eq!(sell.current_trigger_zone.as_deref(), Some("09:30-09:45"));

        let buyback =
            decide_intraday_t_signal(cn_datetime(2026, 7, 7, 13, 40, 0), 0.24, &stats, 0.70, 0.30);
        assert_eq!(buyback.output_state, "buyback_attention");
        assert_eq!(buyback.current_trigger_zone.as_deref(), Some("13:35-14:30"));
    }

    #[test]
    fn legacy_intraday_t_conflicting_conditions_return_watch() {
        let stats = IntradayTWindowStats {
            valid_days: 22,
            high_frequency_windows: vec!["09:30-09:45".into()],
            low_frequency_windows: vec!["09:30-09:45".into()],
        };

        let decision =
            decide_intraday_t_signal(cn_datetime(2026, 7, 7, 9, 35, 0), 0.50, &stats, 0.50, 0.50);

        assert_eq!(decision.output_state, "watch");
        assert_eq!(decision.current_trigger_zone, None);
        assert!(decision.reason.unwrap().contains("冲突"));
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
    fn intraday_t_quote_requires_a_current_timestamp_no_more_than_two_minutes_old() {
        let now = cn_datetime(2026, 7, 7, 10, 0, 0);
        let stale_quote = crate::services::quote::RealtimeStockQuote {
            price: 10.0,
            exchange_timestamp: Some(cn_datetime(2026, 7, 7, 9, 57, 59).timestamp()),
            market_status: Some("5".into()),
            quote_fetched_at: "2026-07-07 10:00:00".into(),
        };
        let current_quote = crate::services::quote::RealtimeStockQuote {
            exchange_timestamp: Some(cn_datetime(2026, 7, 7, 9, 58, 0).timestamp()),
            ..stale_quote.clone()
        };
        let future_quote = crate::services::quote::RealtimeStockQuote {
            exchange_timestamp: Some(cn_datetime(2026, 7, 7, 10, 0, 1).timestamp()),
            ..stale_quote.clone()
        };
        let untimestamped_quote = crate::services::quote::RealtimeStockQuote {
            exchange_timestamp: None,
            ..stale_quote.clone()
        };

        assert!(!quote_is_intraday_t_realtime(&stale_quote, now));
        assert!(quote_is_intraday_t_realtime(&current_quote, now));
        assert!(!quote_is_intraday_t_realtime(&future_quote, now));
        assert!(!quote_is_intraday_t_realtime(&untimestamped_quote, now));
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
