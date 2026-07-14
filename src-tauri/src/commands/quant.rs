use crate::db::now_local;
use crate::models::{
    KlineBar, NewQuantWatchlistItem, QuantDashboard, QuantGeneratedSignal, QuantGridZone,
    QuantRefreshResult, QuantSignal, QuantSignalFilter, QuantStrategySettingsUpdate, QuantTarget,
    QuantWatchlistItem, QuantWatchlistItemUpdate,
};
use crate::services::quant::{
    analyze_intraday_t_windows, china_market_now, classify_trend, completed_daily_bars,
    completed_intraday_bars, decide_guarded_intraday_t_signal, decide_signal, dedupe_key,
    generate_grid_zones, intraday_t_day_position, is_trading_time, next_refresh_at,
    quote_is_intraday_t_realtime, quote_is_strictly_realtime, IntradayTSignalInput,
};
use crate::services::quote::{
    fetch_stock_kline, fetch_stock_realtime_quote, lookup_stock_name, RealtimeStockQuote,
};
use crate::AppState;
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::future::Future;
use tauri::State;

#[derive(Clone)]
pub struct QuantMarketSnapshot {
    code: String,
    market: String,
    daily_bars: Result<Vec<KlineBar>, String>,
    minute_bars: Result<Vec<KlineBar>, String>,
    realtime_quote: Result<RealtimeStockQuote, String>,
}

#[derive(Clone)]
struct QuantSnapshotRequest {
    code: String,
    market: String,
    daily_limit: i64,
    minute_limit: Option<i64>,
}

const DEFAULT_MA_SHORT: usize = 5;
const DEFAULT_MA_LONG: usize = 20;
const DEFAULT_GRID_LOOKBACK_DAYS: usize = 20;
const DEFAULT_POLL_INTERVAL_SECONDS: i64 = 60;
const DEFAULT_STRATEGY_MODE: &str = "auto_grid";
const DEFAULT_INTRADAY_LOOKBACK_DAYS: usize = 22;
const DEFAULT_INTRADAY_TIME_MIN_COUNT: usize = 4;
const DEFAULT_SELL_T_POSITION_THRESHOLD: f64 = 0.70;
const DEFAULT_BUYBACK_POSITION_THRESHOLD: f64 = 0.30;
const INTRADAY_T_MIN_RANGE: f64 = 0.0001;
const SUPPORTED_QUANT_MARKET: &str = "cn";

fn ensure_supported_quant_market(market: &str) -> Result<(), String> {
    if market == SUPPORTED_QUANT_MARKET {
        Ok(())
    } else {
        Err(format!(
            "unsupported market for quant: {market}; currently only cn is supported"
        ))
    }
}

fn validate_quant_strategy_mode(strategy_mode: &str) -> Result<String, String> {
    let strategy_mode = strategy_mode.trim();
    match strategy_mode {
        "auto_grid" | "intraday_t" => Ok(strategy_mode.to_string()),
        _ => Err("invalid strategy_mode; expected auto_grid or intraday_t".to_string()),
    }
}

fn clamp_intraday_lookback_days(value: i64) -> usize {
    if value <= 0 {
        DEFAULT_INTRADAY_LOOKBACK_DAYS
    } else {
        (value as usize).clamp(10, 60)
    }
}

fn clamp_intraday_time_min_count(value: i64, lookback_days: usize) -> usize {
    if value <= 0 {
        DEFAULT_INTRADAY_TIME_MIN_COUNT.min(lookback_days)
    } else {
        (value as usize).clamp(1, lookback_days)
    }
}

fn clamp_intraday_position_threshold(value: f64, default: f64) -> f64 {
    if value.is_finite() && (0.0..=1.0).contains(&value) {
        value
    } else {
        default
    }
}

#[derive(Clone)]
struct QuantStrategySettingsRow {
    ma_short: usize,
    ma_long: usize,
    grid_lookback_days: usize,
    poll_interval_seconds: i64,
    strategy_mode: String,
    intraday_lookback_days: usize,
    intraday_high_time_min_count: usize,
    intraday_low_time_min_count: usize,
    sell_t_position_threshold: f64,
    buyback_position_threshold: f64,
}

fn load_quant_strategy_settings_row(
    conn: &Connection,
    code: &str,
    market: &str,
) -> Result<QuantStrategySettingsRow, String> {
    let existing = conn.query_row(
        "SELECT ma_short, ma_long, grid_lookback_days, poll_interval_seconds, strategy_mode,
                intraday_lookback_days, intraday_high_time_min_count, intraday_low_time_min_count,
                sell_t_position_threshold, buyback_position_threshold
         FROM quant_strategy_settings WHERE code = ?1 AND market = ?2",
        params![code, market],
        |row| {
            let intraday_lookback_days = clamp_intraday_lookback_days(row.get(5)?);
            Ok(QuantStrategySettingsRow {
                ma_short: row.get::<_, i64>(0)?.max(1) as usize,
                ma_long: row.get::<_, i64>(1)?.max(1) as usize,
                grid_lookback_days: row.get::<_, i64>(2)?.max(1) as usize,
                poll_interval_seconds: row.get::<_, i64>(3)?.max(1),
                strategy_mode: row.get(4)?,
                intraday_lookback_days,
                intraday_high_time_min_count: clamp_intraday_time_min_count(
                    row.get(6)?,
                    intraday_lookback_days,
                ),
                intraday_low_time_min_count: clamp_intraday_time_min_count(
                    row.get(7)?,
                    intraday_lookback_days,
                ),
                sell_t_position_threshold: clamp_intraday_position_threshold(
                    row.get(8)?,
                    DEFAULT_SELL_T_POSITION_THRESHOLD,
                ),
                buyback_position_threshold: clamp_intraday_position_threshold(
                    row.get(9)?,
                    DEFAULT_BUYBACK_POSITION_THRESHOLD,
                ),
            })
        },
    );

    match existing {
        Ok(settings) => Ok(settings),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(QuantStrategySettingsRow {
            ma_short: DEFAULT_MA_SHORT,
            ma_long: DEFAULT_MA_LONG,
            grid_lookback_days: DEFAULT_GRID_LOOKBACK_DAYS,
            poll_interval_seconds: DEFAULT_POLL_INTERVAL_SECONDS,
            strategy_mode: DEFAULT_STRATEGY_MODE.to_string(),
            intraday_lookback_days: DEFAULT_INTRADAY_LOOKBACK_DAYS,
            intraday_high_time_min_count: DEFAULT_INTRADAY_TIME_MIN_COUNT,
            intraday_low_time_min_count: DEFAULT_INTRADAY_TIME_MIN_COUNT,
            sell_t_position_threshold: DEFAULT_SELL_T_POSITION_THRESHOLD,
            buyback_position_threshold: DEFAULT_BUYBACK_POSITION_THRESHOLD,
        }),
        Err(err) => Err(err.to_string()),
    }
}

#[derive(Default)]
struct QuantTargetCandidate {
    code: String,
    market: String,
    holding_name: Option<String>,
    watchlist_name: Option<String>,
    watchlist_enabled: Option<bool>,
    has_holding: bool,
    has_watchlist: bool,
}

fn source_for_candidate(candidate: &QuantTargetCandidate) -> &'static str {
    match (candidate.has_holding, candidate.has_watchlist) {
        (true, true) => "holding_watchlist",
        (true, false) => "holding",
        (false, true) => "watchlist",
        (false, false) => "watchlist",
    }
}

fn quant_target_from_candidate(
    conn: &Connection,
    candidate: QuantTargetCandidate,
) -> Result<QuantTarget, String> {
    let source = source_for_candidate(&candidate).to_string();
    let (enabled, desktop_notification_enabled, strategy_mode) = ensure_quant_settings(
        conn,
        &candidate.code,
        &candidate.market,
        &source,
        candidate.watchlist_enabled,
    )?;

    Ok(quant_target_from_candidate_settings(
        candidate,
        source,
        strategy_mode,
        enabled,
        desktop_notification_enabled,
    ))
}

fn quant_target_from_candidate_read_only(
    conn: &Connection,
    candidate: QuantTargetCandidate,
) -> Result<QuantTarget, String> {
    let source = source_for_candidate(&candidate).to_string();
    let existing = conn.query_row(
        "SELECT enabled, desktop_notification_enabled, strategy_mode
         FROM quant_strategy_settings WHERE code = ?1 AND market = ?2",
        params![&candidate.code, &candidate.market],
        |row| {
            Ok((
                row.get::<_, i64>(0)? != 0,
                row.get::<_, i64>(1)? != 0,
                row.get::<_, String>(2)?,
            ))
        },
    );
    let (enabled, desktop_notification_enabled, strategy_mode) = match existing {
        Ok(settings) => settings,
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            let enabled = if source == "watchlist" {
                candidate.watchlist_enabled.unwrap_or(true)
            } else {
                true
            };
            (enabled, true, DEFAULT_STRATEGY_MODE.to_string())
        }
        Err(err) => return Err(err.to_string()),
    };

    Ok(quant_target_from_candidate_settings(
        candidate,
        source,
        strategy_mode,
        enabled,
        desktop_notification_enabled,
    ))
}

fn quant_target_from_candidate_settings(
    candidate: QuantTargetCandidate,
    source: String,
    strategy_mode: String,
    enabled: bool,
    desktop_notification_enabled: bool,
) -> QuantTarget {
    let name = candidate
        .holding_name
        .filter(|name| !name.trim().is_empty())
        .or_else(|| {
            candidate
                .watchlist_name
                .filter(|name| !name.trim().is_empty())
        })
        .unwrap_or_else(|| candidate.code.clone());

    QuantTarget {
        code: candidate.code,
        name,
        market: candidate.market,
        source,
        strategy_mode,
        enabled,
        desktop_notification_enabled,
        current_price: None,
        quote_fetched_at: None,
        trend_state: "insufficient_data".to_string(),
        output_state: "watch".to_string(),
        current_trigger_zone: None,
        ma_short: None,
        ma_long: None,
        grid_zones: vec![],
        latest_signal: None,
        intraday_position: None,
        intraday_reason: None,
        intraday_high_frequency_windows: vec![],
        intraday_low_frequency_windows: vec![],
        has_sold_t_today: false,
        last_error: None,
    }
}

pub fn ensure_quant_settings(
    conn: &Connection,
    code: &str,
    market: &str,
    source: &str,
    watchlist_enabled: Option<bool>,
) -> Result<(bool, bool, String), String> {
    let existing = conn.query_row(
        "SELECT enabled, desktop_notification_enabled, strategy_mode
         FROM quant_strategy_settings WHERE code = ?1 AND market = ?2",
        params![code, market],
        |row| {
            Ok((
                row.get::<_, i64>(0)? != 0,
                row.get::<_, i64>(1)? != 0,
                row.get::<_, String>(2)?,
            ))
        },
    );

    match existing {
        Ok(settings) => Ok(settings),
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            let enabled = if source == "watchlist" {
                watchlist_enabled.unwrap_or(true)
            } else {
                true
            };
            conn.execute(
                "INSERT INTO quant_strategy_settings (code, market, enabled)
                 VALUES (?1, ?2, ?3)",
                params![code, market, if enabled { 1 } else { 0 }],
            )
            .map_err(|e| e.to_string())?;
            Ok((enabled, true, DEFAULT_STRATEGY_MODE.to_string()))
        }
        Err(err) => Err(err.to_string()),
    }
}

fn load_quant_target_candidates(conn: &Connection) -> Result<Vec<QuantTargetCandidate>, String> {
    let mut candidates: HashMap<(String, String), QuantTargetCandidate> = HashMap::new();

    let mut holding_stmt = conn
        .prepare(
            "SELECT code, name, market FROM holdings
             WHERE type = 'stock'
             ORDER BY id ASC",
        )
        .map_err(|e| e.to_string())?;
    let holding_rows = holding_stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    for row in holding_rows {
        let (code, name, market) = row.map_err(|e| e.to_string())?;
        let key = (code.clone(), market.clone());
        let candidate = candidates
            .entry(key)
            .or_insert_with(|| QuantTargetCandidate {
                code,
                market,
                ..Default::default()
            });
        candidate.has_holding = true;
        let name = name.trim();
        if !name.is_empty() {
            candidate.holding_name = Some(name.to_string());
        }
    }

    let mut watchlist_stmt = conn
        .prepare("SELECT code, name, market, enabled FROM quant_watchlist ORDER BY id ASC")
        .map_err(|e| e.to_string())?;
    let watchlist_rows = watchlist_stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)? != 0,
            ))
        })
        .map_err(|e| e.to_string())?;

    for row in watchlist_rows {
        let (code, name, market, enabled) = row.map_err(|e| e.to_string())?;
        let key = (code.clone(), market.clone());
        let candidate = candidates
            .entry(key)
            .or_insert_with(|| QuantTargetCandidate {
                code,
                market,
                ..Default::default()
            });
        candidate.has_watchlist = true;
        let name = name.trim();
        if !name.is_empty() {
            candidate.watchlist_name = Some(name.to_string());
        }
        candidate.watchlist_enabled = Some(enabled);
    }

    let mut candidates = candidates.into_values().collect::<Vec<_>>();
    candidates.sort_by(|a, b| a.market.cmp(&b.market).then(a.code.cmp(&b.code)));
    Ok(candidates)
}

pub fn merge_quant_targets(conn: &Connection) -> Result<Vec<QuantTarget>, String> {
    let mut targets = load_quant_target_candidates(conn)?
        .into_iter()
        .map(|candidate| quant_target_from_candidate(conn, candidate))
        .collect::<Result<Vec<_>, _>>()?;
    set_intraday_t_state_for_date(
        conn,
        &mut targets,
        &china_market_now().format("%Y-%m-%d").to_string(),
    )?;
    targets.sort_by(|a, b| a.market.cmp(&b.market).then(a.code.cmp(&b.code)));
    Ok(targets)
}

fn merge_quant_targets_read_only(conn: &Connection) -> Result<Vec<QuantTarget>, String> {
    let mut targets = load_quant_target_candidates(conn)?
        .into_iter()
        .map(|candidate| quant_target_from_candidate_read_only(conn, candidate))
        .collect::<Result<Vec<_>, _>>()?;
    targets.sort_by(|a, b| a.market.cmp(&b.market).then(a.code.cmp(&b.code)));
    Ok(targets)
}

fn load_quant_target_candidate(
    conn: &Connection,
    code: &str,
    market: &str,
) -> Result<Option<QuantTargetCandidate>, String> {
    let mut candidate = QuantTargetCandidate {
        code: code.to_string(),
        market: market.to_string(),
        ..Default::default()
    };

    let mut holding_stmt = conn
        .prepare(
            "SELECT name FROM holdings
             WHERE code = ?1 AND market = ?2 AND type = 'stock'
             ORDER BY id ASC",
        )
        .map_err(|e| e.to_string())?;
    let holding_rows = holding_stmt
        .query_map(params![code, market], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    for row in holding_rows {
        candidate.has_holding = true;
        let name = row.map_err(|e| e.to_string())?;
        let name = name.trim();
        if !name.is_empty() {
            candidate.holding_name = Some(name.to_string());
        }
    }

    let watchlist = conn.query_row(
        "SELECT name, enabled FROM quant_watchlist WHERE code = ?1 AND market = ?2",
        params![code, market],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? != 0)),
    );
    match watchlist {
        Ok((name, enabled)) => {
            candidate.has_watchlist = true;
            let name = name.trim();
            if !name.is_empty() {
                candidate.watchlist_name = Some(name.to_string());
            }
            candidate.watchlist_enabled = Some(enabled);
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => {}
        Err(err) => return Err(err.to_string()),
    }

    if candidate.has_holding || candidate.has_watchlist {
        Ok(Some(candidate))
    } else {
        Ok(None)
    }
}

fn intraday_t_sold_at_for_date(
    conn: &Connection,
    code: &str,
    market: &str,
    trading_date: &str,
) -> Result<Option<String>, String> {
    match conn.query_row(
        "SELECT sold_at FROM quant_intraday_t_state
         WHERE code = ?1 AND market = ?2 AND trading_date = ?3",
        params![code, market, trading_date],
        |row| row.get(0),
    ) {
        Ok(sold_at) => Ok(Some(sold_at)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(err) => Err(err.to_string()),
    }
}

fn set_intraday_t_state_for_date(
    conn: &Connection,
    targets: &mut [QuantTarget],
    trading_date: &str,
) -> Result<(), String> {
    for target in targets {
        target.has_sold_t_today =
            intraday_t_sold_at_for_date(conn, &target.code, &target.market, trading_date)?
                .is_some();
    }
    Ok(())
}

fn validate_intraday_t_target(conn: &Connection, code: &str, market: &str) -> Result<(), String> {
    ensure_supported_quant_market(market)?;
    load_quant_target_candidate(conn, code, market)?.ok_or_else(|| "量化目标不存在".to_string())?;
    if load_quant_strategy_settings_row(conn, code, market)?.strategy_mode != "intraday_t" {
        return Err("当前量化目标未启用日内T策略".to_string());
    }
    Ok(())
}

fn mark_intraday_t_sold_in_conn(
    conn: &Connection,
    code: &str,
    market: &str,
    quote: &RealtimeStockQuote,
    now: chrono::DateTime<chrono::FixedOffset>,
) -> Result<(), String> {
    let code = code.trim();
    let market = market.trim();
    validate_intraday_t_target(conn, code, market)?;
    if !quote_is_intraday_t_realtime(quote, now) {
        return Err("行情不是当前交易时段实时数据".to_string());
    }

    conn.execute(
        "INSERT INTO quant_intraday_t_state (code, market, trading_date, sold_at)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(code, market, trading_date) DO UPDATE SET sold_at = excluded.sold_at",
        params![
            code,
            market,
            now.format("%Y-%m-%d").to_string(),
            now.format("%Y-%m-%d %H:%M:%S").to_string(),
        ],
    )
    .map_err(|err| err.to_string())?;
    Ok(())
}

fn clear_intraday_t_sold_in_conn(
    conn: &Connection,
    code: &str,
    market: &str,
    quote: &RealtimeStockQuote,
    now: chrono::DateTime<chrono::FixedOffset>,
) -> Result<(), String> {
    let code = code.trim();
    let market = market.trim();
    validate_intraday_t_target(conn, code, market)?;
    if !quote_is_intraday_t_realtime(quote, now) {
        return Err("行情不是当前交易时段实时数据".to_string());
    }

    conn.execute(
        "DELETE FROM quant_intraday_t_state
         WHERE code = ?1 AND market = ?2 AND trading_date = ?3",
        params![code, market, now.format("%Y-%m-%d").to_string()],
    )
    .map_err(|err| err.to_string())?;
    Ok(())
}

fn update_quant_strategy_settings_in_conn(
    conn: &Connection,
    code: String,
    market: String,
    input: QuantStrategySettingsUpdate,
) -> Result<QuantTarget, String> {
    let code = code.trim().to_string();
    let market = market.trim().to_string();
    ensure_supported_quant_market(&market)?;
    let strategy_mode = input
        .strategy_mode
        .as_deref()
        .map(validate_quant_strategy_mode)
        .transpose()?;
    let candidate = load_quant_target_candidate(conn, &code, &market)?
        .ok_or_else(|| "量化目标不存在".to_string())?;

    conn.execute(
        "INSERT INTO quant_strategy_settings
            (code, market, enabled, desktop_notification_enabled, strategy_mode, updated_at)
         VALUES (?1, ?2, ?3, ?4, COALESCE(?5, 'auto_grid'), ?6)
         ON CONFLICT(code, market) DO UPDATE SET
            enabled = excluded.enabled,
            desktop_notification_enabled = excluded.desktop_notification_enabled,
            strategy_mode = COALESCE(?5, quant_strategy_settings.strategy_mode),
            updated_at = excluded.updated_at",
        params![
            &code,
            &market,
            if input.enabled { 1 } else { 0 },
            if input.desktop_notification_enabled {
                1
            } else {
                0
            },
            strategy_mode,
            now_local()
        ],
    )
    .map_err(|e| e.to_string())?;

    quant_target_from_candidate(conn, candidate)
}

fn quant_watchlist_from_row(row: &rusqlite::Row) -> rusqlite::Result<QuantWatchlistItem> {
    let enabled: i64 = row.get(4)?;
    Ok(QuantWatchlistItem {
        id: row.get(0)?,
        code: row.get(1)?,
        name: row.get(2)?,
        market: row.get(3)?,
        enabled: enabled != 0,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

fn quant_signal_from_row(row: &rusqlite::Row) -> rusqlite::Result<QuantSignal> {
    Ok(QuantSignal {
        id: row.get(0)?,
        code: row.get(1)?,
        name: row.get(2)?,
        market: row.get(3)?,
        direction: row.get(4)?,
        trigger_price: row.get(5)?,
        trend_state: row.get(6)?,
        source: row.get(7)?,
        trigger_zone: row.get(8)?,
        triggered_at: row.get(9)?,
    })
}

pub fn list_quant_signals_from_conn(
    conn: &Connection,
    filter: Option<QuantSignalFilter>,
) -> Result<Vec<QuantSignal>, String> {
    let filter = filter.unwrap_or_default();
    let limit = filter
        .limit
        .filter(|limit| *limit > 0)
        .unwrap_or(100)
        .min(500);
    let mut stmt = conn
        .prepare(
            "SELECT id, code, name, market, direction, trigger_price, trend_state, source, trigger_zone, triggered_at
             FROM quant_signals
             WHERE (?1 IS NULL OR code = ?1)
               AND (?2 IS NULL OR direction = ?2)
               AND (?3 IS NULL OR triggered_at >= ?3)
               AND (?4 IS NULL OR triggered_at <= ?4)
             ORDER BY triggered_at DESC, id DESC
             LIMIT ?5",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(
            params![filter.code, filter.direction, filter.from, filter.to, limit],
            quant_signal_from_row,
        )
        .map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

fn latest_quant_signal_from_conn(
    conn: &Connection,
    code: &str,
    market: &str,
) -> Result<Option<QuantSignal>, String> {
    let result = conn.query_row(
        "SELECT id, code, name, market, direction, trigger_price, trend_state, source, trigger_zone, triggered_at
         FROM quant_signals
         WHERE code = ?1 AND market = ?2
         ORDER BY triggered_at DESC, id DESC
         LIMIT 1",
        params![code, market],
        quant_signal_from_row,
    );

    match result {
        Ok(signal) => Ok(Some(signal)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(err) => Err(err.to_string()),
    }
}

fn grid_zones_for_output(
    zones: Vec<crate::services::quant::QuantGridZoneInternal>,
) -> Vec<QuantGridZone> {
    zones
        .into_iter()
        .map(|zone| QuantGridZone {
            zone: zone.zone,
            direction: zone.direction,
            lower: zone.lower,
            upper: zone.upper,
        })
        .collect()
}

pub fn build_quant_dashboard_with_snapshots(
    conn: &Connection,
    snapshots: Vec<QuantMarketSnapshot>,
    now: chrono::DateTime<chrono::FixedOffset>,
) -> Result<QuantDashboard, String> {
    let mut targets = merge_quant_targets_read_only(conn)?;
    let current_date = now.format("%Y-%m-%d").to_string();
    let recent_signals = list_quant_signals_from_conn(
        conn,
        Some(QuantSignalFilter {
            limit: Some(100),
            ..Default::default()
        }),
    )?;
    let snapshot_map: HashMap<(String, String), QuantMarketSnapshot> = snapshots
        .into_iter()
        .map(|snapshot| ((snapshot.code.clone(), snapshot.market.clone()), snapshot))
        .collect();
    let mut dashboard_poll_interval_seconds = None;

    for target in &mut targets {
        target.latest_signal = latest_quant_signal_from_conn(conn, &target.code, &target.market)?;
        let sold_at =
            intraday_t_sold_at_for_date(conn, &target.code, &target.market, &current_date)?;
        target.has_sold_t_today = sold_at.is_some();

        if !target.enabled {
            target.output_state = "watch".to_string();
            target.trend_state = "neutral".to_string();
            continue;
        }

        let Some(snapshot) = snapshot_map.get(&(target.code.clone(), target.market.clone())) else {
            continue;
        };

        let settings = load_quant_strategy_settings_row(conn, &target.code, &target.market)?;
        target.strategy_mode = settings.strategy_mode.clone();
        dashboard_poll_interval_seconds = match dashboard_poll_interval_seconds {
            None => Some(settings.poll_interval_seconds),
            Some(existing) if existing == settings.poll_interval_seconds => Some(existing),
            Some(_) => Some(60),
        };
        let intraday_stats = if settings.strategy_mode == "intraday_t" {
            target.strategy_mode = "intraday_t".to_string();
            target.grid_zones = vec![];

            let minute_bars = match &snapshot.minute_bars {
                Ok(bars) => bars,
                Err(err) => {
                    target.output_state = "quote_error".to_string();
                    target.last_error = Some(err.clone());
                    continue;
                }
            };
            let stats = analyze_intraday_t_windows(
                minute_bars,
                &current_date,
                settings.intraday_lookback_days,
                settings.intraday_high_time_min_count,
                settings.intraday_low_time_min_count,
            );
            target.intraday_high_frequency_windows = stats.high_frequency_windows.clone();
            target.intraday_low_frequency_windows = stats.low_frequency_windows.clone();
            Some(stats)
        } else {
            None
        };
        let bars = match &snapshot.daily_bars {
            Ok(bars) => completed_daily_bars(bars, now),
            Err(err) => {
                target.output_state = "quote_error".to_string();
                target.last_error = Some(err.clone());
                continue;
            }
        };
        let quote = match &snapshot.realtime_quote {
            Ok(quote) => quote,
            Err(err) => {
                target.output_state = "quote_error".to_string();
                target.last_error = Some(err.clone());
                continue;
            }
        };

        target.current_price = Some(quote.price);
        target.quote_fetched_at = Some(quote.quote_fetched_at.clone());

        if !quote_is_strictly_realtime(quote, now) {
            target.output_state = "quote_error".to_string();
            target.last_error = Some("行情不是当前交易时段实时数据".to_string());
            continue;
        }

        if settings.strategy_mode == "intraday_t" && !quote_is_intraday_t_realtime(quote, now) {
            target.output_state = "quote_error".to_string();
            target.last_error = Some("行情不是当前交易时段实时数据".to_string());
            continue;
        }

        let trend = classify_trend(&bars, settings.ma_short, settings.ma_long);
        target.trend_state = trend.state.clone();
        target.ma_short = trend.ma_short;
        target.ma_long = trend.ma_long;

        if settings.strategy_mode == "intraday_t" {
            let stats = intraday_stats.expect("intraday targets analyze minute history first");
            let minute_bars = snapshot
                .minute_bars
                .as_ref()
                .expect("intraday targets require successful minute history");
            let completed_minute_bars = completed_intraday_bars(minute_bars, now);
            target.intraday_position = intraday_t_day_position(
                &completed_minute_bars,
                &current_date,
                quote.price,
                INTRADAY_T_MIN_RANGE,
            );

            if stats.valid_days < 10 {
                target.output_state = "watch".to_string();
                target.current_trigger_zone = None;
                target.intraday_reason = Some("分时统计数据不足".to_string());
                continue;
            }

            let Some(position) = target.intraday_position else {
                target.output_state = "watch".to_string();
                target.current_trigger_zone = None;
                target.intraday_reason = Some("当日波动不足，暂不触发".to_string());
                continue;
            };

            let decision = decide_guarded_intraday_t_signal(IntradayTSignalInput {
                now,
                position,
                stats: &stats,
                sell_threshold: settings.sell_t_position_threshold,
                buyback_threshold: settings.buyback_position_threshold,
                current_price: quote.price,
                minute_bars: &completed_minute_bars,
                sold_at: sold_at.as_deref(),
            });
            target.output_state = decision.output_state;
            target.current_trigger_zone = decision.current_trigger_zone;
            target.intraday_reason = decision
                .reason
                .or_else(|| Some("未进入高发时段或日内位置未达阈值".to_string()));
            continue;
        }

        let Some(zones) = generate_grid_zones(&bars, settings.grid_lookback_days) else {
            target.output_state = "watch".to_string();
            target.last_error = Some("历史数据不足，暂无法生成网格区间".to_string());
            continue;
        };

        if trend.state == "insufficient_data" {
            target.output_state = "watch".to_string();
            target.current_trigger_zone = None;
            target.grid_zones = grid_zones_for_output(zones);
            target.last_error = Some("历史数据不足，暂无法判断趋势".to_string());
            continue;
        }

        let decision = decide_signal(quote.price, &trend.state, &zones);
        target.output_state = decision.output_state;
        target.current_trigger_zone = decision.current_trigger_zone;
        target.grid_zones = grid_zones_for_output(zones);
    }

    Ok(QuantDashboard {
        is_trading_time: is_trading_time(now),
        next_refresh_at: next_refresh_at(now, dashboard_poll_interval_seconds.unwrap_or(60)),
        targets,
        recent_signals,
    })
}

fn latest_signal_time_for_dedupe_key(
    conn: &Connection,
    key: &str,
) -> Result<Option<String>, String> {
    let result = conn.query_row(
        "SELECT triggered_at
         FROM quant_signals
         WHERE dedupe_key = ?1
         ORDER BY triggered_at DESC, id DESC
         LIMIT 1",
        params![key],
        |row| row.get::<_, String>(0),
    );

    match result {
        Ok(triggered_at) => Ok(Some(triggered_at)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(err) => Err(err.to_string()),
    }
}

fn cooldown_allows_signal(
    last_triggered_at: Option<String>,
    now: chrono::DateTime<chrono::FixedOffset>,
) -> bool {
    let Some(last_triggered_at) = last_triggered_at else {
        return true;
    };
    let Ok(last_naive) =
        chrono::NaiveDateTime::parse_from_str(&last_triggered_at, "%Y-%m-%d %H:%M:%S")
    else {
        return true;
    };
    let Some(last) = last_naive.and_local_timezone(*now.offset()).single() else {
        return true;
    };

    now.signed_duration_since(last) >= chrono::Duration::minutes(5)
}

fn insert_quant_signal(
    conn: &Connection,
    target: &QuantTarget,
    source: &str,
    direction: &str,
    trigger_zone: &str,
    key: &str,
    now: chrono::DateTime<chrono::FixedOffset>,
) -> Result<QuantSignal, String> {
    let trigger_price = target
        .current_price
        .ok_or_else(|| "量化信号缺少当前价格".to_string())?;
    let triggered_at = now.format("%Y-%m-%d %H:%M:%S").to_string();

    conn.execute(
        "INSERT INTO quant_signals
            (code, name, market, direction, trigger_price, trend_state, source, trigger_zone, dedupe_key, triggered_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            &target.code,
            &target.name,
            &target.market,
            direction,
            trigger_price,
            &target.trend_state,
            source,
            trigger_zone,
            key,
            &triggered_at,
        ],
    )
    .map_err(|e| e.to_string())?;

    conn.query_row(
        "SELECT id, code, name, market, direction, trigger_price, trend_state, source, trigger_zone, triggered_at
         FROM quant_signals
         WHERE id = ?1",
        params![conn.last_insert_rowid()],
        quant_signal_from_row,
    )
    .map_err(|e| e.to_string())
}

fn refresh_quant_signals_with_snapshots(
    conn: &Connection,
    snapshots: &[QuantMarketSnapshot],
    now: chrono::DateTime<chrono::FixedOffset>,
) -> Result<QuantRefreshResult, String> {
    let dashboard = build_quant_dashboard_with_snapshots(conn, snapshots.to_vec(), now)?;

    if !is_trading_time(now) {
        return Ok(QuantRefreshResult {
            dashboard,
            generated_signals: vec![],
        });
    }

    let mut generated_signals = Vec::new();
    for target in &dashboard.targets {
        if !target.enabled {
            continue;
        }
        if !matches!(
            target.output_state.as_str(),
            "buy_attention" | "sell_attention" | "sell_t_attention" | "buyback_attention"
        ) {
            continue;
        }
        let Some(trigger_zone) = target.current_trigger_zone.as_deref() else {
            continue;
        };

        let key = dedupe_key(
            &target.code,
            &target.market,
            &target.output_state,
            trigger_zone,
        );
        if !cooldown_allows_signal(latest_signal_time_for_dedupe_key(conn, &key)?, now) {
            continue;
        }

        let source = if target.strategy_mode == "intraday_t" {
            "intraday_t"
        } else {
            "auto_grid"
        };
        let signal = insert_quant_signal(
            conn,
            target,
            source,
            &target.output_state,
            trigger_zone,
            &key,
            now,
        )?;
        let notification_body = if source == "intraday_t" {
            target
                .intraday_position
                .map(|position| match target.output_state.as_str() {
                    "sell_t_attention" => format!(
                        "历史高点高发时段 {}，当前日内位置 {:.0}%，可关注卖T。仅供参考。",
                        trigger_zone,
                        position * 100.0
                    ),
                    "buyback_attention" => format!(
                        "历史低点高发时段 {}，当前日内位置 {:.0}%，已满足止跌确认，可关注买回。仅供参考。",
                        trigger_zone,
                        position * 100.0
                    ),
                    _ => format!(
                        "历史高发时段 {}，当前日内位置 {:.0}%，仅供参考。",
                        trigger_zone,
                        position * 100.0
                    ),
                })
        } else {
            None
        };
        generated_signals.push(QuantGeneratedSignal {
            signal,
            desktop_notification_enabled: target.desktop_notification_enabled,
            notification_body,
        });
    }

    Ok(QuantRefreshResult {
        dashboard: build_quant_dashboard_with_snapshots(conn, snapshots.to_vec(), now)?,
        generated_signals,
    })
}

fn should_fetch_intraday_minute_bars(strategy_mode: &str) -> bool {
    strategy_mode == "intraday_t"
}

async fn load_live_quant_snapshots(state: &State<'_, AppState>) -> Result<Vec<QuantMarketSnapshot>, String> {
    let requests = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let targets = merge_quant_targets_read_only(&conn)?;
        let mut requests = Vec::new();

        for target in targets.into_iter().filter(|target| target.enabled) {
            let settings = load_quant_strategy_settings_row(&conn, &target.code, &target.market)?;
            let limit = settings.ma_long.max(settings.grid_lookback_days) as i64 + 1;
            let minute_limit = if settings.strategy_mode == "intraday_t" {
                Some((settings.intraday_lookback_days * 60) as i64)
            } else {
                None
            };
            requests.push(QuantSnapshotRequest {
                code: target.code,
                market: target.market,
                daily_limit: limit,
                minute_limit,
            });
        }

        requests
    };

    Ok(
        fetch_quant_snapshots_concurrently(requests, fetch_live_quant_snapshot).await,
    )
}

async fn fetch_live_quant_snapshot(request: QuantSnapshotRequest) -> QuantMarketSnapshot {
    let daily_bars = fetch_stock_kline(&request.code, "day", request.daily_limit).await;
    let realtime_quote = fetch_stock_realtime_quote(&request.code, &request.market).await;
    let minute_bars = match request.minute_limit {
        Some(limit) if should_fetch_intraday_minute_bars("intraday_t") => {
            fetch_stock_kline(&request.code, "5m", limit).await
        }
        _ => Ok(vec![]),
    };

    QuantMarketSnapshot {
        code: request.code,
        market: request.market,
        daily_bars,
        minute_bars,
        realtime_quote,
    }
}

async fn fetch_quant_snapshots_concurrently<F, Fut>(
    requests: Vec<QuantSnapshotRequest>,
    fetch: F,
) -> Vec<QuantMarketSnapshot>
where
    F: Fn(QuantSnapshotRequest) -> Fut + Copy + Send + Sync + 'static,
    Fut: Future<Output = QuantMarketSnapshot> + Send + 'static,
{
    let mut handles = Vec::with_capacity(requests.len());
    for request in requests {
        handles.push(tokio::spawn(fetch(request)));
    }

    let mut snapshots = Vec::with_capacity(handles.len());
    for handle in handles {
        if let Ok(snapshot) = handle.await {
            snapshots.push(snapshot);
        }
    }
    snapshots
}

fn get_watchlist_by_id(conn: &Connection, id: i64) -> Result<QuantWatchlistItem, String> {
    conn.query_row(
        "SELECT id, code, name, market, enabled, created_at, updated_at
         FROM quant_watchlist WHERE id = ?1",
        params![id],
        quant_watchlist_from_row,
    )
    .map_err(|e| e.to_string())
}

fn map_insert_error(err: rusqlite::Error) -> String {
    let message = err.to_string();
    if message.contains("UNIQUE constraint failed") {
        "该代码和市场的量化自选已存在 (already exists)".to_string()
    } else {
        message
    }
}

fn add_watchlist_to_conn(
    conn: &Connection,
    input: NewQuantWatchlistItem,
) -> Result<QuantWatchlistItem, String> {
    let code = input.code.trim().to_string();
    if code.is_empty() {
        return Err("代码不能为空".to_string());
    }

    let market = input
        .market
        .as_deref()
        .map(str::trim)
        .filter(|market| !market.is_empty())
        .unwrap_or("cn")
        .to_string();
    ensure_supported_quant_market(&market)?;
    let name = input
        .name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or(&code)
        .to_string();
    let enabled = if input.enabled.unwrap_or(true) { 1 } else { 0 };

    conn.execute(
        "INSERT INTO quant_watchlist (code, name, market, enabled)
         VALUES (?1, ?2, ?3, ?4)",
        params![code, name, market, enabled],
    )
    .map_err(map_insert_error)?;

    get_watchlist_by_id(conn, conn.last_insert_rowid())
}

fn update_watchlist_in_conn(
    conn: &Connection,
    id: i64,
    input: QuantWatchlistItemUpdate,
) -> Result<QuantWatchlistItem, String> {
    let enabled = if input.enabled { 1 } else { 0 };
    let affected = conn
        .execute(
            "UPDATE quant_watchlist SET name = ?1, enabled = ?2, updated_at = ?3 WHERE id = ?4",
            params![input.name.trim(), enabled, now_local(), id],
        )
        .map_err(|e| e.to_string())?;
    if affected == 0 {
        return Err("量化自选不存在".to_string());
    }

    get_watchlist_by_id(conn, id)
}

fn delete_watchlist_from_conn(conn: &Connection, id: i64) -> Result<(), String> {
    conn.execute("DELETE FROM quant_watchlist WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn list_watchlist_from_conn(conn: &Connection) -> Result<Vec<QuantWatchlistItem>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, code, name, market, enabled, created_at, updated_at
             FROM quant_watchlist ORDER BY id DESC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], quant_watchlist_from_row)
        .map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_quant_watchlist(state: State<AppState>) -> Result<Vec<QuantWatchlistItem>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    list_watchlist_from_conn(&conn)
}

#[tauri::command]
pub fn list_quant_targets(state: State<AppState>) -> Result<Vec<QuantTarget>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    merge_quant_targets(&conn)
}

#[tauri::command]
pub fn list_quant_signals(
    state: State<AppState>,
    filter: Option<QuantSignalFilter>,
) -> Result<Vec<QuantSignal>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    list_quant_signals_from_conn(&conn, filter)
}

#[tauri::command]
pub async fn get_quant_dashboard(state: State<'_, AppState>) -> Result<QuantDashboard, String> {
    let snapshots = load_live_quant_snapshots(&state).await?;
    let now = china_market_now();
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    build_quant_dashboard_with_snapshots(&conn, snapshots, now)
}

#[tauri::command]
pub async fn refresh_quant_signals(
    state: State<'_, AppState>,
) -> Result<QuantRefreshResult, String> {
    let snapshots = load_live_quant_snapshots(&state).await?;
    let now = china_market_now();
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    refresh_quant_signals_with_snapshots(&conn, &snapshots, now)
}

#[tauri::command]
pub async fn mark_intraday_t_sold(
    state: State<'_, AppState>,
    code: String,
    market: String,
) -> Result<(), String> {
    let code = code.trim().to_string();
    let market = market.trim().to_string();
    {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        validate_intraday_t_target(&conn, &code, &market)?;
    }

    let quote = fetch_stock_realtime_quote(&code, &market).await?;
    let now = china_market_now();
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    mark_intraday_t_sold_in_conn(&conn, &code, &market, &quote, now)
}

#[tauri::command]
pub async fn clear_intraday_t_sold(
    state: State<'_, AppState>,
    code: String,
    market: String,
) -> Result<(), String> {
    let code = code.trim().to_string();
    let market = market.trim().to_string();
    {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        validate_intraday_t_target(&conn, &code, &market)?;
    }

    let quote = fetch_stock_realtime_quote(&code, &market).await?;
    let now = china_market_now();
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    clear_intraday_t_sold_in_conn(&conn, &code, &market, &quote, now)
}

#[tauri::command]
pub async fn add_quant_watchlist(
    state: State<'_, AppState>,
    mut input: NewQuantWatchlistItem,
) -> Result<QuantWatchlistItem, String> {
    if input.code.trim().is_empty() {
        return Err("代码不能为空".to_string());
    }

    if input
        .name
        .as_deref()
        .map(str::trim)
        .unwrap_or("")
        .is_empty()
    {
        let code = input.code.trim().to_string();
        input.name = Some(lookup_stock_name(&code).await.unwrap_or(code));
    }

    let conn = state.db.lock().map_err(|e| e.to_string())?;
    add_watchlist_to_conn(&conn, input)
}

#[tauri::command]
pub fn update_quant_watchlist(
    state: State<AppState>,
    id: i64,
    input: QuantWatchlistItemUpdate,
) -> Result<QuantWatchlistItem, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    update_watchlist_in_conn(&conn, id, input)
}

#[tauri::command]
pub fn delete_quant_watchlist(state: State<AppState>, id: i64) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    delete_watchlist_from_conn(&conn, id)
}

#[tauri::command]
pub fn update_quant_strategy_settings(
    state: State<AppState>,
    code: String,
    market: String,
    input: QuantStrategySettingsUpdate,
) -> Result<QuantTarget, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    update_quant_strategy_settings_in_conn(&conn, code, market, input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::KlineBar;
    use crate::services::quant::dedupe_key;
    use crate::services::quote::RealtimeStockQuote;
    use chrono::{FixedOffset, TimeZone};

    fn create_test_schema(conn: &Connection) {
        conn.execute_batch(
            "
            CREATE TABLE accounts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                type TEXT NOT NULL,
                balance REAL NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
            );

            CREATE TABLE categories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                type TEXT NOT NULL,
                icon TEXT NOT NULL DEFAULT ''
            );

            CREATE TABLE transactions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                type TEXT NOT NULL,
                amount REAL NOT NULL,
                category_id INTEGER,
                account_id INTEGER,
                transfer_to_account_id INTEGER,
                note TEXT DEFAULT '',
                transaction_date TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
            );

            CREATE TABLE holdings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                code TEXT NOT NULL,
                name TEXT NOT NULL,
                type TEXT NOT NULL DEFAULT 'stock',
                quantity REAL NOT NULL,
                cost_price REAL NOT NULL,
                current_price REAL NOT NULL DEFAULT 0,
                market TEXT NOT NULL DEFAULT 'cn',
                created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
            );

            CREATE TABLE price_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                holding_id INTEGER NOT NULL,
                price REAL NOT NULL,
                recorded_at TEXT NOT NULL
            );

            CREATE TABLE settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE TABLE quant_watchlist (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                code TEXT NOT NULL,
                name TEXT NOT NULL,
                market TEXT NOT NULL DEFAULT 'cn',
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
                UNIQUE(code, market)
            );

            CREATE TABLE quant_strategy_settings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                code TEXT NOT NULL,
                market TEXT NOT NULL DEFAULT 'cn',
                ma_short INTEGER NOT NULL DEFAULT 5,
                ma_long INTEGER NOT NULL DEFAULT 20,
                grid_lookback_days INTEGER NOT NULL DEFAULT 20,
                poll_interval_seconds INTEGER NOT NULL DEFAULT 60,
                desktop_notification_enabled INTEGER NOT NULL DEFAULT 1,
                enabled INTEGER NOT NULL DEFAULT 1,
                strategy_mode TEXT NOT NULL DEFAULT 'auto_grid',
                intraday_lookback_days INTEGER NOT NULL DEFAULT 22,
                intraday_high_time_min_count INTEGER NOT NULL DEFAULT 4,
                intraday_low_time_min_count INTEGER NOT NULL DEFAULT 4,
                sell_t_position_threshold REAL NOT NULL DEFAULT 0.70,
                buyback_position_threshold REAL NOT NULL DEFAULT 0.30,
                created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
                UNIQUE(code, market)
            );

            CREATE TABLE quant_signals (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                code TEXT NOT NULL,
                name TEXT NOT NULL,
                market TEXT NOT NULL DEFAULT 'cn',
                direction TEXT NOT NULL,
                trigger_price REAL NOT NULL,
                trend_state TEXT NOT NULL,
                source TEXT NOT NULL,
                trigger_zone TEXT NOT NULL,
                dedupe_key TEXT NOT NULL,
                triggered_at TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
            );

            CREATE TABLE quant_intraday_t_state (
                code TEXT NOT NULL,
                market TEXT NOT NULL,
                trading_date TEXT NOT NULL,
                sold_at TEXT NOT NULL,
                PRIMARY KEY (code, market, trading_date)
            );
            ",
        )
        .unwrap();
    }

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        create_test_schema(&conn);
        conn
    }

    #[test]
    fn add_watchlist_rejects_duplicate_code_market() {
        let conn = test_conn();
        let input = NewQuantWatchlistItem {
            code: "000001".to_string(),
            name: Some("平安银行".to_string()),
            market: Some("cn".to_string()),
            enabled: None,
        };

        add_watchlist_to_conn(&conn, input.clone()).unwrap();
        let err = add_watchlist_to_conn(&conn, input).unwrap_err();

        assert!(
            err.contains("already exists") || err.contains("duplicate") || err.contains("已存在"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn add_watchlist_rejects_empty_code() {
        let conn = test_conn();

        let err = add_watchlist_to_conn(
            &conn,
            NewQuantWatchlistItem {
                code: "   ".to_string(),
                name: Some("空代码".to_string()),
                market: None,
                enabled: None,
            },
        )
        .unwrap_err();

        assert!(
            err.contains("code") || err.contains("代码"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn add_watchlist_rejects_unsupported_market() {
        let conn = test_conn();

        let err = add_watchlist_to_conn(
            &conn,
            NewQuantWatchlistItem {
                code: "00700".to_string(),
                name: Some("腾讯控股".to_string()),
                market: Some("hk".to_string()),
                enabled: None,
            },
        )
        .unwrap_err();

        assert!(
            err.contains("unsupported market") || err.contains("市场"),
            "unexpected error: {err}"
        );
        let watchlist_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM quant_watchlist", [], |row| row.get(0))
            .unwrap();
        assert_eq!(watchlist_count, 0);
    }

    #[test]
    fn update_watchlist_changes_name_and_initial_enabled() {
        let conn = test_conn();
        let item = add_watchlist_to_conn(
            &conn,
            NewQuantWatchlistItem {
                code: "513500".to_string(),
                name: Some("标普 ETF".to_string()),
                market: None,
                enabled: Some(false),
            },
        )
        .unwrap();

        assert_eq!(item.market, "cn");
        assert!(!item.enabled);

        let updated = update_watchlist_in_conn(
            &conn,
            item.id,
            QuantWatchlistItemUpdate {
                name: "标普500 ETF".to_string(),
                enabled: true,
            },
        )
        .unwrap();

        assert_eq!(updated.id, item.id);
        assert_eq!(updated.code, "513500");
        assert_eq!(updated.market, "cn");
        assert_eq!(updated.name, "标普500 ETF");
        assert!(updated.enabled);
    }

    fn insert_holding(conn: &Connection, code: &str, name: &str, market: &str, r#type: &str) {
        conn.execute(
            "INSERT INTO holdings (code, name, type, quantity, cost_price, market)
             VALUES (?1, ?2, ?3, 10.0, 1.0, ?4)",
            params![code, name, r#type, market],
        )
        .unwrap();
    }

    fn insert_watchlist(conn: &Connection, code: &str, name: &str, market: &str, enabled: bool) {
        conn.execute(
            "INSERT INTO quant_watchlist (code, name, market, enabled)
             VALUES (?1, ?2, ?3, ?4)",
            params![code, name, market, if enabled { 1 } else { 0 }],
        )
        .unwrap();
    }

    fn insert_settings(
        conn: &Connection,
        code: &str,
        market: &str,
        enabled: bool,
        desktop_notification_enabled: bool,
    ) {
        conn.execute(
            "INSERT INTO quant_strategy_settings
                (code, market, enabled, desktop_notification_enabled)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                code,
                market,
                if enabled { 1 } else { 0 },
                if desktop_notification_enabled { 1 } else { 0 }
            ],
        )
        .unwrap();
    }

    fn insert_settings_with_mode(
        conn: &Connection,
        code: &str,
        market: &str,
        enabled: bool,
        desktop_notification_enabled: bool,
        strategy_mode: &str,
    ) {
        conn.execute(
            "INSERT INTO quant_strategy_settings
                (code, market, enabled, desktop_notification_enabled, strategy_mode)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                code,
                market,
                if enabled { 1 } else { 0 },
                if desktop_notification_enabled { 1 } else { 0 },
                strategy_mode,
            ],
        )
        .unwrap();
    }

    fn insert_intraday_settings(conn: &Connection, code: &str) {
        insert_settings_with_mode(conn, code, "cn", true, true, "intraday_t");
    }

    fn insert_intraday_settings_with_thresholds(
        conn: &Connection,
        code: &str,
        sell_threshold: f64,
        buyback_threshold: f64,
    ) {
        conn.execute(
            "INSERT INTO quant_strategy_settings
                (code, market, enabled, desktop_notification_enabled, strategy_mode,
                 intraday_lookback_days, intraday_high_time_min_count, intraday_low_time_min_count,
                 sell_t_position_threshold, buyback_position_threshold)
             VALUES (?1, 'cn', 1, 1, 'intraday_t', 22, 4, 4, ?2, ?3)",
            params![code, sell_threshold, buyback_threshold],
        )
        .unwrap();
    }

    fn insert_signal(
        conn: &Connection,
        code: &str,
        name: &str,
        direction: &str,
        triggered_at: &str,
    ) {
        let trigger_zone = if direction == "buy_attention" {
            "buy_2"
        } else if direction.contains("sell") {
            "sell_1"
        } else {
            "buy_1"
        };
        conn.execute(
            "INSERT INTO quant_signals
                (code, name, market, direction, trigger_price, trend_state, source, trigger_zone, dedupe_key, triggered_at)
             VALUES (?1, ?2, 'cn', ?3, 10.5, 'bullish', 'watchlist', ?4, ?5, ?6)",
            params![
                code,
                name,
                direction,
                trigger_zone,
                dedupe_key(code, "cn", direction, trigger_zone),
                triggered_at
            ],
        )
        .unwrap();
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

    fn bar(date: &str, close: f64) -> KlineBar {
        KlineBar {
            date: date.to_string(),
            open: close,
            close,
            low: close - 1.0,
            high: close + 1.0,
            volume: 1_000.0,
            change_pct: 0.0,
        }
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
                    20.0
                } else {
                    12.0 + index as f64 * 0.01
                };
                let low = if *time == low_time {
                    1.0
                } else {
                    8.0 + index as f64 * 0.01
                };
                minute_bar(&format!("{date} {time}"), 10.0, high, low)
            })
            .collect()
    }

    fn intraday_t_history(high_time: &str, low_time: &str, days: usize) -> Vec<KlineBar> {
        let mut bars = Vec::new();
        for day in 1..=days {
            bars.extend(intraday_t_valid_day(
                &format!("2026-06-{day:02}"),
                high_time,
                low_time,
            ));
        }
        bars
    }

    fn intraday_t_current_day(low: f64, high: f64) -> Vec<KlineBar> {
        vec![minute_bar("2026-07-07 09:35", 10.0, high, low)]
    }

    fn intraday_t_minute_bars(
        high_time: &str,
        low_time: &str,
        days: usize,
        current_low: f64,
        current_high: f64,
    ) -> Vec<KlineBar> {
        let mut bars = intraday_t_history(high_time, low_time, days);
        bars.extend(intraday_t_current_day(current_low, current_high));
        bars
    }

    fn intraday_t_buyback_minute_bars() -> Vec<KlineBar> {
        let mut bars = intraday_t_history("09:35", "13:40", 10);
        bars.extend([
            minute_bar("2026-07-07 09:30", 10.0, 12.0, 10.0),
            minute_bar("2026-07-07 09:35", 10.0, 11.0, 10.0),
            minute_bar("2026-07-07 13:00", 10.0, 11.0, 10.2),
        ]);
        bars
    }

    fn insert_intraday_t_sale_state(conn: &Connection, code: &str, sold_at: &str) {
        conn.execute(
            "INSERT INTO quant_intraday_t_state (code, market, trading_date, sold_at)
             VALUES (?1, 'cn', '2026-07-07', ?2)",
            params![code, sold_at],
        )
        .unwrap();
    }

    fn daily_bars_from_closes(closes: &[f64]) -> Vec<KlineBar> {
        closes
            .iter()
            .enumerate()
            .map(|(index, close)| bar(&format!("2026-06-{:02}", index + 1), *close))
            .collect()
    }

    fn bullish_daily_bars() -> Vec<KlineBar> {
        daily_bars_from_closes(&[
            9.0, 9.1, 9.2, 9.3, 9.4, 9.5, 9.6, 9.7, 9.8, 9.9, 10.0, 10.1, 10.2, 10.3, 10.4, 10.5,
            10.6, 10.7, 10.8, 10.9,
        ])
    }

    fn bearish_daily_bars() -> Vec<KlineBar> {
        daily_bars_from_closes(&[
            20.0, 19.8, 19.6, 19.4, 19.2, 19.0, 18.8, 18.6, 18.4, 18.2, 18.0, 17.8, 17.6, 17.4,
            17.2, 17.0, 16.8, 16.6, 16.4, 16.2,
        ])
    }

    fn bars_for_buy_attention() -> Vec<KlineBar> {
        vec![
            bar("2026-06-04", 9.6),
            bar("2026-06-05", 9.8),
            bar("2026-06-08", 10.0),
            bar("2026-06-09", 10.1),
            bar("2026-06-10", 10.2),
            bar("2026-06-11", 10.3),
            bar("2026-06-12", 10.4),
            bar("2026-06-15", 10.5),
            bar("2026-06-16", 10.6),
            bar("2026-06-18", 10.0),
            bar("2026-06-19", 10.2),
            bar("2026-06-22", 10.4),
            bar("2026-06-23", 10.6),
            bar("2026-06-24", 10.8),
            bar("2026-06-25", 11.0),
            bar("2026-06-26", 11.2),
            bar("2026-06-29", 11.4),
            bar("2026-06-30", 11.6),
            bar("2026-07-01", 12.0),
            bar("2026-07-02", 12.2),
            bar("2026-07-03", 12.4),
        ]
    }

    fn realtime_quote(price: f64) -> RealtimeStockQuote {
        RealtimeStockQuote {
            price,
            exchange_timestamp: None,
            market_status: Some("5".to_string()),
            quote_fetched_at: "2026-07-03 10:10:00".to_string(),
        }
    }

    fn realtime_quote_with_status(
        price: f64,
        exchange_timestamp: Option<i64>,
        market_status: Option<&str>,
    ) -> RealtimeStockQuote {
        RealtimeStockQuote {
            price,
            exchange_timestamp,
            market_status: market_status.map(str::to_string),
            quote_fetched_at: "2026-07-03 10:10:00".to_string(),
        }
    }

    fn exchange_timestamp(
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        minute: u32,
        second: u32,
    ) -> i64 {
        cn_datetime(year, month, day, hour, minute, second).timestamp()
    }

    fn snapshot(
        code: &str,
        market: &str,
        daily_bars: Result<Vec<KlineBar>, String>,
        realtime_quote: Result<RealtimeStockQuote, String>,
    ) -> QuantMarketSnapshot {
        QuantMarketSnapshot {
            code: code.to_string(),
            market: market.to_string(),
            daily_bars,
            minute_bars: Ok(vec![]),
            realtime_quote,
        }
    }

    fn snapshot_with_minute_bars(
        code: &str,
        daily_bars: Vec<KlineBar>,
        minute_bars: Result<Vec<KlineBar>, String>,
        price: f64,
        now: chrono::DateTime<chrono::FixedOffset>,
    ) -> QuantMarketSnapshot {
        QuantMarketSnapshot {
            code: code.to_string(),
            market: "cn".to_string(),
            daily_bars: Ok(daily_bars),
            minute_bars,
            realtime_quote: Ok(realtime_quote_with_status(
                price,
                Some(now.timestamp()),
                Some("5"),
            )),
        }
    }

    fn signal_count(conn: &Connection) -> i64 {
        conn.query_row("SELECT COUNT(*) FROM quant_signals", [], |row| row.get(0))
            .unwrap()
    }

    fn latest_signal_row(conn: &Connection) -> QuantSignal {
        conn.query_row(
            "SELECT id, code, name, market, direction, trigger_price, trend_state, source, trigger_zone, triggered_at
             FROM quant_signals
             ORDER BY id DESC
             LIMIT 1",
            [],
            quant_signal_from_row,
        )
        .unwrap()
    }

    fn refresh_buy_snapshot(code: &str) -> QuantMarketSnapshot {
        snapshot(
            code,
            "cn",
            Ok(bars_for_buy_attention()),
            Ok(realtime_quote(11.0)),
        )
    }

    fn settings_count(conn: &Connection) -> i64 {
        conn.query_row("SELECT COUNT(*) FROM quant_strategy_settings", [], |row| {
            row.get(0)
        })
        .unwrap()
    }

    #[test]
    fn intraday_t_sale_acknowledgement_marks_dashboard_state_and_can_be_cleared() {
        let conn = test_conn();
        insert_watchlist(&conn, "000107", "T State", "cn", true);
        insert_intraday_settings(&conn, "000107");
        let now = cn_datetime(2026, 7, 7, 10, 0, 0);
        let quote = realtime_quote_with_status(10.0, Some(now.timestamp()), Some("5"));

        mark_intraday_t_sold_in_conn(&conn, "000107", "cn", &quote, now).unwrap();

        let dashboard = build_quant_dashboard_with_snapshots(&conn, vec![], now).unwrap();
        assert!(dashboard.targets[0].has_sold_t_today);
        let sold_at: String = conn
            .query_row(
                "SELECT sold_at FROM quant_intraday_t_state
                 WHERE code = '000107' AND market = 'cn' AND trading_date = '2026-07-07'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(sold_at, "2026-07-07 10:00:00");

        clear_intraday_t_sold_in_conn(&conn, "000107", "cn", &quote, now).unwrap();

        let dashboard = build_quant_dashboard_with_snapshots(&conn, vec![], now).unwrap();
        assert!(!dashboard.targets[0].has_sold_t_today);
    }

    #[test]
    fn mark_intraday_t_sold_rejects_stale_quote_without_inserting_state() {
        let conn = test_conn();
        insert_watchlist(&conn, "000118", "Stale Mark", "cn", true);
        insert_intraday_settings(&conn, "000118");
        let now = cn_datetime(2026, 7, 7, 10, 0, 0);
        let quote = realtime_quote_with_status(
            10.0,
            Some(exchange_timestamp(2026, 7, 6, 10, 0, 0)),
            Some("5"),
        );

        let err = mark_intraday_t_sold_in_conn(&conn, "000118", "cn", &quote, now).unwrap_err();

        assert!(err.contains("实时数据"), "unexpected error: {err}");
        let state_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM quant_intraday_t_state", [], |row| row.get(0))
            .unwrap();
        assert_eq!(state_count, 0);
    }

    #[test]
    fn mark_intraday_t_sold_rejects_unsupported_market_without_inserting_state() {
        let conn = test_conn();
        let now = cn_datetime(2026, 7, 7, 10, 0, 0);
        let quote = realtime_quote_with_status(10.0, Some(now.timestamp()), Some("5"));

        let err = mark_intraday_t_sold_in_conn(&conn, "00700", "hk", &quote, now).unwrap_err();

        assert!(err.contains("unsupported market"), "unexpected error: {err}");
        let state_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM quant_intraday_t_state", [], |row| row.get(0))
            .unwrap();
        assert_eq!(state_count, 0);
    }

    #[test]
    fn mark_intraday_t_sold_rejects_missing_target_without_inserting_state() {
        let conn = test_conn();
        let now = cn_datetime(2026, 7, 7, 10, 0, 0);
        let quote = realtime_quote_with_status(10.0, Some(now.timestamp()), Some("5"));

        let err = mark_intraday_t_sold_in_conn(&conn, "000119", "cn", &quote, now).unwrap_err();

        assert!(err.contains("量化目标不存在"), "unexpected error: {err}");
        let state_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM quant_intraday_t_state", [], |row| row.get(0))
            .unwrap();
        assert_eq!(state_count, 0);
    }

    #[test]
    fn mark_intraday_t_sold_rejects_auto_grid_target_without_inserting_state() {
        let conn = test_conn();
        insert_watchlist(&conn, "000120", "Auto Grid Mark", "cn", true);
        let now = cn_datetime(2026, 7, 7, 10, 0, 0);
        let quote = realtime_quote_with_status(10.0, Some(now.timestamp()), Some("5"));

        let err = mark_intraday_t_sold_in_conn(&conn, "000120", "cn", &quote, now).unwrap_err();

        assert!(err.contains("未启用日内T策略"), "unexpected error: {err}");
        let state_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM quant_intraday_t_state", [], |row| row.get(0))
            .unwrap();
        assert_eq!(state_count, 0);
    }

    #[test]
    fn clear_intraday_t_sold_rejects_stale_quote_without_deleting_same_day_state() {
        let conn = test_conn();
        insert_watchlist(&conn, "000121", "Stale Clear", "cn", true);
        insert_intraday_settings(&conn, "000121");
        insert_intraday_t_sale_state(&conn, "000121", "2026-07-07 09:45:00");
        let now = cn_datetime(2026, 7, 7, 10, 0, 0);
        let quote = realtime_quote_with_status(
            10.0,
            Some(exchange_timestamp(2026, 7, 6, 10, 0, 0)),
            Some("5"),
        );

        let err = clear_intraday_t_sold_in_conn(&conn, "000121", "cn", &quote, now).unwrap_err();

        assert!(err.contains("实时数据"), "unexpected error: {err}");
        let sold_at: String = conn
            .query_row(
                "SELECT sold_at FROM quant_intraday_t_state
                 WHERE code = '000121' AND market = 'cn' AND trading_date = '2026-07-07'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(sold_at, "2026-07-07 09:45:00");
    }

    #[test]
    fn dashboard_does_not_persist_signals() {
        let conn = test_conn();
        insert_holding(&conn, "600000", "浦发银行", "cn", "stock");
        let now = cn_datetime(2026, 7, 3, 10, 15, 0);

        let dashboard = build_quant_dashboard_with_snapshots(
            &conn,
            vec![snapshot(
                "600000",
                "cn",
                Ok(bars_for_buy_attention()),
                Ok(realtime_quote(11.0)),
            )],
            now,
        )
        .unwrap();

        assert_eq!(dashboard.targets.len(), 1);
        assert_eq!(dashboard.targets[0].output_state, "buy_attention");
        assert!(dashboard.targets[0].quote_fetched_at.is_some());
        assert_eq!(signal_count(&conn), 0);
    }

    #[test]
    fn dashboard_does_not_insert_missing_strategy_settings() {
        let conn = test_conn();
        insert_holding(&conn, "600000", "浦发银行", "cn", "stock");
        let now = cn_datetime(2026, 7, 3, 10, 15, 0);

        let dashboard = build_quant_dashboard_with_snapshots(&conn, vec![], now).unwrap();

        assert_eq!(dashboard.targets.len(), 1);
        assert_eq!(dashboard.targets[0].code, "600000");
        assert!(dashboard.targets[0].enabled);
        assert!(dashboard.targets[0].desktop_notification_enabled);
        assert_eq!(settings_count(&conn), 0);
    }

    #[test]
    fn disabled_dashboard_target_is_watch_and_neutral_without_snapshot() {
        let conn = test_conn();
        insert_holding(&conn, "600000", "浦发银行", "cn", "stock");
        insert_settings(&conn, "600000", "cn", false, true);
        let now = cn_datetime(2026, 7, 3, 10, 15, 0);

        let dashboard = build_quant_dashboard_with_snapshots(&conn, vec![], now).unwrap();

        assert_eq!(dashboard.targets.len(), 1);
        assert_eq!(dashboard.targets[0].output_state, "watch");
        assert_eq!(dashboard.targets[0].trend_state, "neutral");
        assert!(dashboard.targets[0].quote_fetched_at.is_none());
        assert!(dashboard.targets[0].current_price.is_none());
    }

    #[test]
    fn per_target_quote_error_does_not_hide_other_targets() {
        let conn = test_conn();
        insert_holding(&conn, "600000", "浦发银行", "cn", "stock");
        insert_holding(&conn, "000001", "平安银行", "cn", "stock");
        let now = cn_datetime(2026, 7, 3, 10, 15, 0);

        let dashboard = build_quant_dashboard_with_snapshots(
            &conn,
            vec![
                snapshot(
                    "600000",
                    "cn",
                    Ok(bars_for_buy_attention()),
                    Err("quote failed".to_string()),
                ),
                snapshot(
                    "000001",
                    "cn",
                    Ok(bars_for_buy_attention()),
                    Ok(realtime_quote(20.0)),
                ),
            ],
            now,
        )
        .unwrap();

        assert_eq!(dashboard.targets.len(), 2);
        let failed = dashboard
            .targets
            .iter()
            .find(|target| target.code == "600000")
            .unwrap();
        let watched = dashboard
            .targets
            .iter()
            .find(|target| target.code == "000001")
            .unwrap();
        assert_eq!(failed.output_state, "quote_error");
        assert_eq!(failed.last_error.as_deref(), Some("quote failed"));
        assert_eq!(watched.output_state, "watch");
    }

    #[test]
    fn intraday_t_dashboard_emits_sell_attention_with_reason_and_position() {
        let conn = test_conn();
        insert_watchlist(&conn, "000101", "Sell T", "cn", true);
        insert_intraday_settings(&conn, "000101");
        let now = cn_datetime(2026, 7, 7, 9, 36, 0);

        let dashboard = build_quant_dashboard_with_snapshots(
            &conn,
            vec![snapshot_with_minute_bars(
                "000101",
                bullish_daily_bars(),
                Ok(intraday_t_minute_bars("09:35", "13:40", 10, 10.0, 12.0)),
                11.6,
                now,
            )],
            now,
        )
        .unwrap();

        let target = &dashboard.targets[0];
        assert_eq!(target.strategy_mode, "intraday_t");
        assert_eq!(target.output_state, "sell_t_attention");
        assert_eq!(target.current_trigger_zone.as_deref(), Some("09:30-09:45"));
        assert!(target.grid_zones.is_empty());
        assert!(target
            .intraday_high_frequency_windows
            .contains(&"09:30-09:45".to_string()));
        assert!((target.intraday_position.unwrap() - 0.8).abs() < 1e-9);
        let reason = target.intraday_reason.as_deref().unwrap();
        assert!(reason.contains("卖T确认"));
        assert!(reason.contains("历史"));
    }

    #[test]
    fn intraday_t_dashboard_emits_buyback_attention_with_reason_and_position() {
        let conn = test_conn();
        insert_watchlist(&conn, "000102", "Buyback", "cn", true);
        insert_intraday_settings(&conn, "000102");
        insert_intraday_t_sale_state(&conn, "000102", "2026-07-07 09:00:00");
        let now = cn_datetime(2026, 7, 7, 13, 40, 0);

        let dashboard = build_quant_dashboard_with_snapshots(
            &conn,
            vec![snapshot_with_minute_bars(
                "000102",
                bullish_daily_bars(),
                Ok(intraday_t_buyback_minute_bars()),
                10.4,
                now,
            )],
            now,
        )
        .unwrap();

        let target = &dashboard.targets[0];
        assert_eq!(target.output_state, "buyback_attention");
        assert_eq!(target.current_trigger_zone.as_deref(), Some("13:35-14:30"));
        assert!(target
            .intraday_low_frequency_windows
            .contains(&"13:35-14:30".to_string()));
        assert!((target.intraday_position.unwrap() - 0.2).abs() < 1e-9);
        let reason = target.intraday_reason.as_deref().unwrap();
        assert!(reason.contains("买回确认"));
        assert!(reason.contains("历史"));
    }

    #[test]
    fn intraday_t_dashboard_keeps_windows_when_quote_is_stale() {
        let conn = test_conn();
        insert_watchlist(&conn, "000122", "Stale Windows", "cn", true);
        insert_intraday_settings(&conn, "000122");
        let now = cn_datetime(2026, 7, 7, 9, 36, 0);

        let dashboard = build_quant_dashboard_with_snapshots(
            &conn,
            vec![QuantMarketSnapshot {
                code: "000122".to_string(),
                market: "cn".to_string(),
                daily_bars: Ok(bullish_daily_bars()),
                minute_bars: Ok(intraday_t_minute_bars("09:35", "13:40", 10, 10.0, 12.0)),
                realtime_quote: Ok(realtime_quote_with_status(
                    11.6,
                    Some(exchange_timestamp(2026, 7, 6, 9, 35, 0)),
                    Some("5"),
                )),
            }],
            now,
        )
        .unwrap();

        let target = &dashboard.targets[0];
        assert_eq!(target.output_state, "quote_error");
        assert!(target
            .intraday_high_frequency_windows
            .contains(&"09:30-09:45".to_string()));
        assert!(target
            .intraday_low_frequency_windows
            .contains(&"13:35-14:30".to_string()));
    }

    #[test]
    fn intraday_t_dashboard_keeps_windows_when_daily_history_fails() {
        let conn = test_conn();
        insert_watchlist(&conn, "000123", "Daily Failure Windows", "cn", true);
        insert_intraday_settings(&conn, "000123");
        let now = cn_datetime(2026, 7, 7, 9, 36, 0);

        let dashboard = build_quant_dashboard_with_snapshots(
            &conn,
            vec![QuantMarketSnapshot {
                code: "000123".to_string(),
                market: "cn".to_string(),
                daily_bars: Err("daily history failed".to_string()),
                minute_bars: Ok(intraday_t_minute_bars("09:35", "13:40", 10, 10.0, 12.0)),
                realtime_quote: Ok(realtime_quote_with_status(
                    11.6,
                    Some(now.timestamp()),
                    Some("5"),
                )),
            }],
            now,
        )
        .unwrap();

        let target = &dashboard.targets[0];
        assert_eq!(target.output_state, "quote_error");
        assert_eq!(target.last_error.as_deref(), Some("daily history failed"));
        assert!(target
            .intraday_high_frequency_windows
            .contains(&"09:30-09:45".to_string()));
        assert!(target
            .intraday_low_frequency_windows
            .contains(&"13:35-14:30".to_string()));
    }

    #[test]
    fn intraday_t_dashboard_requires_persisted_sale_before_buyback() {
        let conn = test_conn();
        insert_watchlist(&conn, "000108", "Guarded Buyback", "cn", true);
        insert_intraday_settings(&conn, "000108");
        let now = cn_datetime(2026, 7, 7, 9, 41, 0);
        let mut minute_bars = intraday_t_history("10:00", "09:35", 10);
        minute_bars.extend([
            minute_bar("2026-07-07 09:30", 100.0, 101.0, 100.0),
            minute_bar("2026-07-07 09:35", 100.0, 101.0, 100.0),
            minute_bar("2026-07-07 09:40", 100.0, 102.0, 100.2),
        ]);

        let dashboard = build_quant_dashboard_with_snapshots(
            &conn,
            vec![snapshot_with_minute_bars(
                "000108",
                bullish_daily_bars(),
                Ok(minute_bars),
                100.6,
                now,
            )],
            now,
        )
        .unwrap();

        assert!(!dashboard.targets[0].has_sold_t_today);
        assert_eq!(dashboard.targets[0].output_state, "watch");
    }

    #[test]
    fn intraday_t_insufficient_history_stays_watch_with_reason() {
        let conn = test_conn();
        insert_watchlist(&conn, "000103", "Short History", "cn", true);
        insert_intraday_settings(&conn, "000103");
        let now = cn_datetime(2026, 7, 7, 9, 35, 0);

        let dashboard = build_quant_dashboard_with_snapshots(
            &conn,
            vec![snapshot_with_minute_bars(
                "000103",
                bullish_daily_bars(),
                Ok(intraday_t_minute_bars("09:35", "13:40", 9, 10.0, 12.0)),
                11.6,
                now,
            )],
            now,
        )
        .unwrap();

        let target = &dashboard.targets[0];
        assert_eq!(target.output_state, "watch");
        assert_eq!(target.current_trigger_zone, None);
        assert_eq!(target.intraday_reason.as_deref(), Some("分时统计数据不足"));
    }

    #[test]
    fn intraday_t_narrow_current_range_stays_watch_with_reason() {
        let conn = test_conn();
        insert_watchlist(&conn, "000104", "Narrow", "cn", true);
        insert_intraday_settings(&conn, "000104");
        let now = cn_datetime(2026, 7, 7, 9, 35, 0);

        let dashboard = build_quant_dashboard_with_snapshots(
            &conn,
            vec![snapshot_with_minute_bars(
                "000104",
                bullish_daily_bars(),
                Ok(intraday_t_minute_bars("09:35", "13:40", 10, 10.0, 10.00001)),
                10.000005,
                now,
            )],
            now,
        )
        .unwrap();

        let target = &dashboard.targets[0];
        assert_eq!(target.output_state, "watch");
        assert_eq!(target.current_trigger_zone, None);
        assert_eq!(
            target.intraday_reason.as_deref(),
            Some("当日波动不足，暂不触发")
        );
    }

    #[test]
    fn intraday_t_ordinary_watch_includes_display_reason() {
        let conn = test_conn();
        insert_watchlist(&conn, "000105", "Watch", "cn", true);
        insert_intraday_settings(&conn, "000105");
        let now = cn_datetime(2026, 7, 7, 10, 0, 0);

        let dashboard = build_quant_dashboard_with_snapshots(
            &conn,
            vec![snapshot_with_minute_bars(
                "000105",
                bullish_daily_bars(),
                Ok(intraday_t_minute_bars("09:35", "13:40", 10, 10.0, 12.0)),
                11.0,
                now,
            )],
            now,
        )
        .unwrap();

        let target = &dashboard.targets[0];
        assert_eq!(target.output_state, "watch");
        assert_eq!(target.current_trigger_zone, None);
        assert_eq!(
            target.intraday_reason.as_deref(),
            Some("缺少日内T确认数据，暂不触发")
        );
    }

    #[test]
    fn intraday_t_weak_bearish_trend_does_not_block_sell_t() {
        let conn = test_conn();
        insert_watchlist(&conn, "000106", "Weak", "cn", true);
        insert_intraday_settings(&conn, "000106");
        let now = cn_datetime(2026, 7, 7, 9, 36, 0);

        let dashboard = build_quant_dashboard_with_snapshots(
            &conn,
            vec![snapshot_with_minute_bars(
                "000106",
                bearish_daily_bars(),
                Ok(intraday_t_minute_bars("09:35", "13:40", 10, 10.0, 12.0)),
                11.6,
                now,
            )],
            now,
        )
        .unwrap();

        assert_eq!(dashboard.targets[0].trend_state, "bearish");
        assert_eq!(dashboard.targets[0].output_state, "sell_t_attention");
    }

    #[test]
    fn dashboard_populates_recent_and_latest_signals_from_history_without_persisting() {
        let conn = test_conn();
        insert_holding(&conn, "600000", "浦发银行", "cn", "stock");
        insert_holding(&conn, "000001", "平安银行", "cn", "stock");
        insert_signal(&conn, "600000", "浦发银行", "buy", "2026-07-01 10:00:00");
        insert_signal(&conn, "000001", "平安银行", "sell", "2026-07-01 11:00:00");
        insert_signal(&conn, "600000", "浦发银行", "sell", "2026-07-01 12:00:00");
        let count_before = signal_count(&conn);
        let now = cn_datetime(2026, 7, 3, 10, 15, 0);

        let dashboard = build_quant_dashboard_with_snapshots(&conn, vec![], now).unwrap();

        assert_eq!(dashboard.recent_signals.len(), 3);
        assert_eq!(dashboard.recent_signals[0].code, "600000");
        assert_eq!(dashboard.recent_signals[0].direction, "sell");
        let first = dashboard
            .targets
            .iter()
            .find(|target| target.code == "600000")
            .unwrap();
        let second = dashboard
            .targets
            .iter()
            .find(|target| target.code == "000001")
            .unwrap();
        assert_eq!(first.latest_signal.as_ref().unwrap().direction, "sell");
        assert_eq!(second.latest_signal.as_ref().unwrap().direction, "sell");
        assert_eq!(signal_count(&conn), count_before);
    }

    #[test]
    fn market_closed_refresh_persists_no_signals() {
        let conn = test_conn();
        insert_holding(&conn, "600000", "浦发银行", "cn", "stock");
        let now = cn_datetime(2026, 7, 3, 15, 1, 0);
        let snapshots = vec![refresh_buy_snapshot("600000")];

        let result = refresh_quant_signals_with_snapshots(&conn, &snapshots, now).unwrap();

        assert!(!result.dashboard.is_trading_time);
        assert_eq!(result.dashboard.targets[0].output_state, "quote_error");
        assert!(result.generated_signals.is_empty());
        assert_eq!(signal_count(&conn), 0);
    }

    #[test]
    fn refresh_persists_generated_signal_after_cooldown() {
        let conn = test_conn();
        insert_holding(&conn, "600000", "浦发银行", "cn", "stock");
        insert_signal(
            &conn,
            "600000",
            "浦发银行",
            "buy_attention",
            "2026-07-03 10:00:00",
        );
        let now = cn_datetime(2026, 7, 3, 10, 5, 1);
        let snapshots = vec![refresh_buy_snapshot("600000")];

        let result = refresh_quant_signals_with_snapshots(&conn, &snapshots, now).unwrap();

        assert_eq!(result.generated_signals.len(), 1);
        let generated = &result.generated_signals[0].signal;
        assert_eq!(generated.code, "600000");
        assert_eq!(generated.name, "浦发银行");
        assert_eq!(generated.market, "cn");
        assert_eq!(generated.direction, "buy_attention");
        assert_eq!(generated.trigger_price, 11.0);
        assert_eq!(generated.trend_state, "bullish");
        assert_eq!(generated.source, "auto_grid");
        assert_eq!(generated.trigger_zone, "buy_2");
        assert_eq!(generated.triggered_at, "2026-07-03 10:05:01");
        assert_eq!(signal_count(&conn), 2);
        assert_eq!(latest_signal_row(&conn).source, "auto_grid");
        assert_eq!(result.dashboard.recent_signals[0].id, generated.id);
        assert_eq!(
            result.dashboard.targets[0]
                .latest_signal
                .as_ref()
                .unwrap()
                .id,
            generated.id
        );
    }

    #[test]
    fn refresh_suppresses_duplicate_within_five_minutes() {
        let conn = test_conn();
        insert_holding(&conn, "600000", "浦发银行", "cn", "stock");
        insert_signal(
            &conn,
            "600000",
            "浦发银行",
            "buy_attention",
            "2026-07-03 10:00:00",
        );
        let before = signal_count(&conn);
        let now = cn_datetime(2026, 7, 3, 10, 4, 59);
        let snapshots = vec![refresh_buy_snapshot("600000")];

        let result = refresh_quant_signals_with_snapshots(&conn, &snapshots, now).unwrap();

        assert!(result.generated_signals.is_empty());
        assert_eq!(signal_count(&conn), before);
    }

    #[test]
    fn refresh_allows_same_identity_after_five_minutes() {
        let conn = test_conn();
        insert_holding(&conn, "600000", "浦发银行", "cn", "stock");
        insert_signal(
            &conn,
            "600000",
            "浦发银行",
            "buy_attention",
            "2026-07-03 10:00:00",
        );
        let now = cn_datetime(2026, 7, 3, 10, 5, 0);
        let snapshots = vec![refresh_buy_snapshot("600000")];

        let result = refresh_quant_signals_with_snapshots(&conn, &snapshots, now).unwrap();

        assert_eq!(result.generated_signals.len(), 1);
        assert_eq!(signal_count(&conn), 2);
    }

    #[test]
    fn cooldown_identity_differs_by_market_direction_and_zone() {
        let conn = test_conn();
        insert_holding(&conn, "600000", "浦发银行", "cn", "stock");
        conn.execute(
            "INSERT INTO quant_signals
                (code, name, market, direction, trigger_price, trend_state, source, trigger_zone, dedupe_key, triggered_at)
             VALUES
                ('600000', '浦发银行', 'hk', 'buy_attention', 11.0, 'bullish', 'auto_grid', 'buy_1', 'hk:600000:buy_attention:buy_1', '2026-07-03 10:01:00'),
                ('600000', '浦发银行', 'cn', 'sell_attention', 11.0, 'neutral', 'auto_grid', 'sell_1', 'cn:600000:sell_attention:sell_1', '2026-07-03 10:01:00'),
                ('600000', '浦发银行', 'cn', 'buy_attention', 11.0, 'bullish', 'auto_grid', 'buy_1', 'cn:600000:buy_attention:buy_1', '2026-07-03 10:01:00')",
            [],
        )
        .unwrap();
        let now = cn_datetime(2026, 7, 3, 10, 3, 0);
        let snapshots = vec![refresh_buy_snapshot("600000")];

        let result = refresh_quant_signals_with_snapshots(&conn, &snapshots, now).unwrap();

        assert_eq!(result.generated_signals.len(), 1);
        assert_eq!(result.generated_signals[0].signal.trigger_zone, "buy_2");
        assert_eq!(signal_count(&conn), 4);
    }

    #[test]
    fn generated_signal_carries_desktop_notification_enabled_flag() {
        let conn = test_conn();
        insert_watchlist(&conn, "000006", "Quiet Watch", "cn", true);
        insert_settings(&conn, "000006", "cn", true, false);
        let now = cn_datetime(2026, 7, 3, 10, 15, 0);
        let snapshots = vec![refresh_buy_snapshot("000006")];

        let result = refresh_quant_signals_with_snapshots(&conn, &snapshots, now).unwrap();

        assert_eq!(result.generated_signals.len(), 1);
        assert!(!result.generated_signals[0].desktop_notification_enabled);
        assert_eq!(signal_count(&conn), 1);
    }

    #[test]
    fn quote_error_persists_no_signal_and_does_not_stop_other_targets() {
        let conn = test_conn();
        insert_holding(&conn, "600000", "浦发银行", "cn", "stock");
        insert_holding(&conn, "000001", "平安银行", "cn", "stock");
        let now = cn_datetime(2026, 7, 3, 10, 15, 0);
        let snapshots = vec![
            snapshot(
                "600000",
                "cn",
                Ok(bars_for_buy_attention()),
                Err("quote failed".to_string()),
            ),
            refresh_buy_snapshot("000001"),
        ];

        let result = refresh_quant_signals_with_snapshots(&conn, &snapshots, now).unwrap();

        assert_eq!(result.generated_signals.len(), 1);
        assert_eq!(result.generated_signals[0].signal.code, "000001");
        assert_eq!(signal_count(&conn), 1);
        let failed = result
            .dashboard
            .targets
            .iter()
            .find(|target| target.code == "600000")
            .unwrap();
        assert_eq!(failed.output_state, "quote_error");
        assert_eq!(failed.last_error.as_deref(), Some("quote failed"));
    }

    #[test]
    fn strict_realtime_missing_timestamp_or_status_generates_no_signal() {
        let conn = test_conn();
        insert_holding(&conn, "600000", "浦发银行", "cn", "stock");
        let now = cn_datetime(2026, 7, 3, 10, 15, 0);
        let snapshots = vec![snapshot(
            "600000",
            "cn",
            Ok(bars_for_buy_attention()),
            Ok(realtime_quote_with_status(11.0, None, None)),
        )];

        let result = refresh_quant_signals_with_snapshots(&conn, &snapshots, now).unwrap();

        assert!(result.generated_signals.is_empty());
        assert_eq!(result.dashboard.targets[0].output_state, "quote_error");
        assert_eq!(signal_count(&conn), 0);
    }

    #[test]
    fn strict_realtime_stale_or_wrong_session_timestamp_generates_no_signal() {
        let conn = test_conn();
        insert_holding(&conn, "600000", "浦发银行", "cn", "stock");
        insert_holding(&conn, "000001", "平安银行", "cn", "stock");
        let now = cn_datetime(2026, 7, 3, 10, 15, 0);
        let snapshots = vec![
            snapshot(
                "600000",
                "cn",
                Ok(bars_for_buy_attention()),
                Ok(realtime_quote_with_status(
                    11.0,
                    Some(exchange_timestamp(2026, 7, 2, 10, 15, 0)),
                    Some("5"),
                )),
            ),
            snapshot(
                "000001",
                "cn",
                Ok(bars_for_buy_attention()),
                Ok(realtime_quote_with_status(
                    11.0,
                    Some(exchange_timestamp(2026, 7, 3, 12, 0, 0)),
                    Some("5"),
                )),
            ),
        ];

        let result = refresh_quant_signals_with_snapshots(&conn, &snapshots, now).unwrap();

        assert!(result.generated_signals.is_empty());
        assert_eq!(signal_count(&conn), 0);
        assert!(result
            .dashboard
            .targets
            .iter()
            .all(|target| target.output_state == "quote_error"));
    }

    #[test]
    fn strict_realtime_stale_timestamp_with_open_status_still_generates_no_signal() {
        let conn = test_conn();
        insert_holding(&conn, "600000", "浦发银行", "cn", "stock");
        let now = cn_datetime(2026, 7, 3, 10, 15, 0);
        let snapshots = vec![snapshot(
            "600000",
            "cn",
            Ok(bars_for_buy_attention()),
            Ok(realtime_quote_with_status(
                11.0,
                Some(exchange_timestamp(2026, 7, 2, 10, 15, 0)),
                Some("5"),
            )),
        )];

        let result = refresh_quant_signals_with_snapshots(&conn, &snapshots, now).unwrap();

        assert!(result.generated_signals.is_empty());
        assert_eq!(signal_count(&conn), 0);
    }

    #[test]
    fn strict_realtime_closed_status_generates_no_signal() {
        let conn = test_conn();
        insert_holding(&conn, "600000", "浦发银行", "cn", "stock");
        let now = cn_datetime(2026, 7, 3, 10, 15, 0);
        let snapshots = vec![snapshot(
            "600000",
            "cn",
            Ok(bars_for_buy_attention()),
            Ok(realtime_quote_with_status(11.0, None, Some("3"))),
        )];

        let result = refresh_quant_signals_with_snapshots(&conn, &snapshots, now).unwrap();

        assert!(result.generated_signals.is_empty());
        assert_eq!(signal_count(&conn), 0);
    }

    #[test]
    fn strict_realtime_current_session_timestamp_allows_signal() {
        let conn = test_conn();
        insert_holding(&conn, "600000", "浦发银行", "cn", "stock");
        let now = cn_datetime(2026, 7, 3, 10, 15, 0);
        let snapshots = vec![snapshot(
            "600000",
            "cn",
            Ok(bars_for_buy_attention()),
            Ok(realtime_quote_with_status(
                11.0,
                Some(exchange_timestamp(2026, 7, 3, 10, 14, 0)),
                Some("3"),
            )),
        )];

        let result = refresh_quant_signals_with_snapshots(&conn, &snapshots, now).unwrap();

        assert_eq!(result.generated_signals.len(), 1);
        assert_eq!(signal_count(&conn), 1);
    }

    #[test]
    fn intraday_t_refresh_persists_intraday_signal_source_and_time_bucket() {
        let conn = test_conn();
        insert_watchlist(&conn, "000107", "Refresh T", "cn", true);
        insert_intraday_settings(&conn, "000107");
        let now = cn_datetime(2026, 7, 7, 9, 36, 0);
        let snapshots = vec![snapshot_with_minute_bars(
            "000107",
            bullish_daily_bars(),
            Ok(intraday_t_minute_bars("09:35", "13:40", 10, 10.0, 12.0)),
            11.6,
            now,
        )];

        let result = refresh_quant_signals_with_snapshots(&conn, &snapshots, now).unwrap();

        assert_eq!(result.generated_signals.len(), 1);
        let generated = &result.generated_signals[0];
        assert_eq!(generated.signal.direction, "sell_t_attention");
        assert_eq!(generated.signal.source, "intraday_t");
        assert_eq!(generated.signal.trigger_zone, "09:30-09:45");
        let body = generated.notification_body.as_deref().unwrap();
        assert!(body.contains("历史高点高发时段"));
        assert!(body.contains("09:30-09:45"));
        assert!(body.contains("80%"));
        assert!(body.contains("可关注卖T"));
        assert_eq!(signal_count(&conn), 1);
        let persisted = latest_signal_row(&conn);
        assert_eq!(persisted.source, "intraday_t");
        assert_eq!(persisted.trigger_zone, "09:30-09:45");
    }

    #[test]
    fn intraday_t_refresh_does_not_persist_or_generate_sell_t_watch() {
        let conn = test_conn();
        insert_watchlist(&conn, "000117", "Opening Watch", "cn", true);
        insert_intraday_settings(&conn, "000117");
        let now = cn_datetime(2026, 7, 7, 9, 34, 0);
        let mut minute_bars = intraday_t_history("09:35", "13:40", 10);
        minute_bars.push(minute_bar("2026-07-07 09:30", 10.0, 12.0, 10.0));
        let snapshots = vec![snapshot_with_minute_bars(
            "000117",
            bullish_daily_bars(),
            Ok(minute_bars),
            11.6,
            now,
        )];

        let result = refresh_quant_signals_with_snapshots(&conn, &snapshots, now).unwrap();

        assert_eq!(result.dashboard.targets[0].output_state, "sell_t_watch");
        assert!(result.generated_signals.is_empty());
        assert_eq!(signal_count(&conn), 0);
    }

    #[test]
    fn intraday_t_refresh_buyback_notification_body_names_low_frequency_window() {
        let conn = test_conn();
        insert_watchlist(&conn, "000116", "Buyback Body", "cn", true);
        insert_intraday_settings(&conn, "000116");
        insert_intraday_t_sale_state(&conn, "000116", "2026-07-07 09:00:00");
        let now = cn_datetime(2026, 7, 7, 13, 40, 0);
        let snapshots = vec![snapshot_with_minute_bars(
            "000116",
            bullish_daily_bars(),
            Ok(intraday_t_buyback_minute_bars()),
            10.4,
            now,
        )];

        let result = refresh_quant_signals_with_snapshots(&conn, &snapshots, now).unwrap();

        assert_eq!(result.generated_signals.len(), 1);
        let generated = &result.generated_signals[0];
        assert_eq!(generated.signal.direction, "buyback_attention");
        assert_eq!(generated.signal.trigger_zone, "13:35-14:30");
        let body = generated.notification_body.as_deref().unwrap();
        assert!(body.contains("历史低点高发时段"));
        assert!(body.contains("13:35-14:30"));
        assert!(body.contains("20%"));
        assert!(body.contains("已满足止跌确认"));
        assert!(body.contains("可关注买回"));
    }

    #[test]
    fn intraday_t_refresh_rejects_stale_or_non_realtime_quote() {
        let conn = test_conn();
        insert_watchlist(&conn, "000108", "Stale T", "cn", true);
        insert_intraday_settings(&conn, "000108");
        let now = cn_datetime(2026, 7, 7, 9, 36, 0);
        let snapshots = vec![QuantMarketSnapshot {
            code: "000108".to_string(),
            market: "cn".to_string(),
            daily_bars: Ok(bullish_daily_bars()),
            minute_bars: Ok(intraday_t_minute_bars("09:35", "13:40", 10, 10.0, 12.0)),
            realtime_quote: Ok(realtime_quote_with_status(
                11.6,
                Some(exchange_timestamp(2026, 7, 6, 9, 35, 0)),
                Some("5"),
            )),
        }];

        let result = refresh_quant_signals_with_snapshots(&conn, &snapshots, now).unwrap();

        assert!(result.generated_signals.is_empty());
        assert_eq!(result.dashboard.targets[0].output_state, "quote_error");
        assert_eq!(signal_count(&conn), 0);
    }

    #[test]
    fn intraday_t_minute_fetch_does_not_require_a_realtime_quote() {
        assert!(should_fetch_intraday_minute_bars("intraday_t"));
        assert!(!should_fetch_intraday_minute_bars("auto_grid"));
    }

    #[test]
    fn intraday_t_market_closed_persists_no_generated_signal() {
        let conn = test_conn();
        insert_watchlist(&conn, "000109", "Closed T", "cn", true);
        insert_intraday_settings(&conn, "000109");
        let now = cn_datetime(2026, 7, 7, 15, 1, 0);
        let snapshots = vec![snapshot_with_minute_bars(
            "000109",
            bullish_daily_bars(),
            Ok(intraday_t_minute_bars("09:35", "13:40", 10, 10.0, 12.0)),
            11.6,
            cn_datetime(2026, 7, 7, 9, 36, 0),
        )];

        let result = refresh_quant_signals_with_snapshots(&conn, &snapshots, now).unwrap();

        assert!(result.generated_signals.is_empty());
        assert_eq!(signal_count(&conn), 0);
    }

    #[test]
    fn intraday_t_cooldown_dedupes_same_stock_market_direction_time_bucket() {
        let conn = test_conn();
        insert_watchlist(&conn, "000110", "Cooldown T", "cn", true);
        insert_intraday_settings(&conn, "000110");
        let snapshots = vec![snapshot_with_minute_bars(
            "000110",
            bullish_daily_bars(),
            Ok(intraday_t_minute_bars("09:35", "13:40", 10, 10.0, 12.0)),
            11.6,
            cn_datetime(2026, 7, 7, 9, 36, 0),
        )];

        let first = refresh_quant_signals_with_snapshots(
            &conn,
            &snapshots,
            cn_datetime(2026, 7, 7, 9, 36, 0),
        )
        .unwrap();
        let second = refresh_quant_signals_with_snapshots(
            &conn,
            &snapshots,
            cn_datetime(2026, 7, 7, 9, 39, 0),
        )
        .unwrap();

        assert_eq!(first.generated_signals.len(), 1);
        assert!(second.generated_signals.is_empty());
        assert_eq!(signal_count(&conn), 1);
    }

    #[test]
    fn intraday_t_cooldown_allows_different_direction_or_time_bucket() {
        let conn = test_conn();
        insert_watchlist(&conn, "000111", "Direction T", "cn", true);
        insert_intraday_settings(&conn, "000111");
        conn.execute(
            "INSERT INTO quant_signals
                (code, name, market, direction, trigger_price, trend_state, source, trigger_zone, dedupe_key, triggered_at)
              VALUES ('000111', 'Direction T', 'cn', 'buyback_attention', 11.6, 'bullish', 'intraday_t', '09:30-09:45', ?1, '2026-07-07 09:34:00')",
            params![dedupe_key("000111", "cn", "buyback_attention", "09:30-09:45")],
        )
        .unwrap();
        let direction_result = refresh_quant_signals_with_snapshots(
            &conn,
            &[snapshot_with_minute_bars(
                "000111",
                bullish_daily_bars(),
                Ok(intraday_t_minute_bars("09:35", "13:40", 10, 10.0, 12.0)),
                11.6,
                cn_datetime(2026, 7, 7, 9, 36, 0),
            )],
            cn_datetime(2026, 7, 7, 9, 36, 0),
        )
        .unwrap();
        assert_eq!(direction_result.generated_signals.len(), 1);
        assert_eq!(
            direction_result.generated_signals[0].signal.direction,
            "sell_t_attention"
        );

        let conn = test_conn();
        insert_watchlist(&conn, "000112", "Bucket T", "cn", true);
        insert_intraday_settings(&conn, "000112");
        conn.execute(
            "INSERT INTO quant_signals
                (code, name, market, direction, trigger_price, trend_state, source, trigger_zone, dedupe_key, triggered_at)
             VALUES ('000112', 'Bucket T', 'cn', 'sell_t_attention', 11.6, 'bullish', 'intraday_t', '09:30-09:45', ?1, '2026-07-07 09:46:00')",
            params![dedupe_key("000112", "cn", "sell_t_attention", "09:30-09:45")],
        )
        .unwrap();
        let bucket_result = refresh_quant_signals_with_snapshots(
            &conn,
            &[snapshot_with_minute_bars(
                "000112",
                bullish_daily_bars(),
                Ok(intraday_t_minute_bars("09:50", "13:40", 10, 10.0, 12.0)),
                11.6,
                cn_datetime(2026, 7, 7, 9, 51, 0),
            )],
            cn_datetime(2026, 7, 7, 9, 51, 0),
        )
        .unwrap();
        assert_eq!(bucket_result.generated_signals.len(), 1);
        assert_eq!(
            bucket_result.generated_signals[0].signal.trigger_zone,
            "09:50-10:30"
        );
    }

    #[test]
    fn intraday_t_contradictory_conditions_stay_watch_with_defensive_reason() {
        let conn = test_conn();
        insert_watchlist(&conn, "000113", "Conflict T", "cn", true);
        insert_intraday_settings_with_thresholds(&conn, "000113", 0.5, 0.5);
        let now = cn_datetime(2026, 7, 7, 9, 35, 0);

        let dashboard = build_quant_dashboard_with_snapshots(
            &conn,
            vec![snapshot_with_minute_bars(
                "000113",
                bullish_daily_bars(),
                Ok(intraday_t_minute_bars("09:35", "09:35", 10, 10.0, 12.0)),
                11.0,
                now,
            )],
            now,
        )
        .unwrap();

        let target = &dashboard.targets[0];
        assert_eq!(target.output_state, "watch");
        assert_eq!(target.current_trigger_zone, None);
        assert!(target.intraday_reason.is_some());
    }

    #[test]
    fn intraday_t_and_auto_grid_modes_are_isolated() {
        let conn = test_conn();
        insert_watchlist(&conn, "000114", "Auto", "cn", true);
        insert_watchlist(&conn, "000115", "Do T", "cn", true);
        insert_intraday_settings(&conn, "000115");
        let now = cn_datetime(2026, 7, 7, 9, 36, 0);

        let dashboard = build_quant_dashboard_with_snapshots(
            &conn,
            vec![
                refresh_buy_snapshot("000114"),
                snapshot_with_minute_bars(
                    "000115",
                    bullish_daily_bars(),
                    Ok(intraday_t_minute_bars("09:35", "13:40", 10, 10.0, 12.0)),
                    11.6,
                    now,
                ),
            ],
            now,
        )
        .unwrap();

        let auto = dashboard
            .targets
            .iter()
            .find(|target| target.code == "000114")
            .unwrap();
        let intraday = dashboard
            .targets
            .iter()
            .find(|target| target.code == "000115")
            .unwrap();

        assert_eq!(auto.strategy_mode, "auto_grid");
        assert!(!auto.grid_zones.is_empty());
        assert!(auto.intraday_reason.is_none());
        assert_eq!(intraday.strategy_mode, "intraday_t");
        assert_eq!(intraday.output_state, "sell_t_attention");
        assert!(intraday.grid_zones.is_empty());
    }

    #[tokio::test]
    async fn quant_snapshot_fetches_targets_concurrently() {
        use std::time::{Duration, Instant};

        let requests = vec![
            QuantSnapshotRequest {
                code: "000201".to_string(),
                market: "cn".to_string(),
                daily_limit: 21,
                minute_limit: None,
            },
            QuantSnapshotRequest {
                code: "000202".to_string(),
                market: "cn".to_string(),
                daily_limit: 21,
                minute_limit: None,
            },
            QuantSnapshotRequest {
                code: "000203".to_string(),
                market: "cn".to_string(),
                daily_limit: 21,
                minute_limit: None,
            },
        ];

        let start = Instant::now();
        let snapshots = fetch_quant_snapshots_concurrently(requests, |request| async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            QuantMarketSnapshot {
                code: request.code,
                market: request.market,
                daily_bars: Ok(bullish_daily_bars()),
                minute_bars: Ok(vec![]),
                realtime_quote: Ok(realtime_quote(10.0)),
            }
        })
        .await;

        assert_eq!(snapshots.len(), 3);
        assert!(
            start.elapsed() < Duration::from_millis(220),
            "expected concurrent fetches, took {:?}",
            start.elapsed()
        );
    }

    #[test]
    fn strict_realtime_open_market_status_without_timestamp_allows_signal() {
        let conn = test_conn();
        insert_holding(&conn, "600000", "浦发银行", "cn", "stock");
        let now = cn_datetime(2026, 7, 3, 10, 15, 0);
        let snapshots = vec![snapshot(
            "600000",
            "cn",
            Ok(bars_for_buy_attention()),
            Ok(realtime_quote_with_status(11.0, None, Some("交易中"))),
        )];

        let result = refresh_quant_signals_with_snapshots(&conn, &snapshots, now).unwrap();

        assert_eq!(result.generated_signals.len(), 1);
        assert_eq!(signal_count(&conn), 1);
    }

    #[test]
    fn disabled_targets_are_not_polled_or_persisted() {
        let conn = test_conn();
        insert_holding(&conn, "600000", "浦发银行", "cn", "stock");
        insert_settings(&conn, "600000", "cn", false, true);
        let now = cn_datetime(2026, 7, 3, 10, 15, 0);
        let snapshots = vec![refresh_buy_snapshot("600000")];

        let result = refresh_quant_signals_with_snapshots(&conn, &snapshots, now).unwrap();

        assert!(result.generated_signals.is_empty());
        assert_eq!(signal_count(&conn), 0);
        assert_eq!(result.dashboard.targets[0].output_state, "watch");
        assert!(result.dashboard.targets[0].current_price.is_none());
    }

    #[test]
    fn refresh_does_not_mutate_finance_tables() {
        let conn = test_conn();
        conn.execute(
            "INSERT INTO accounts (name, type, balance) VALUES ('Cash', 'asset', 123.45)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO categories (name, type, icon) VALUES ('Food', 'expense', 'food')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO transactions (type, amount, category_id, account_id, note, transaction_date)
             VALUES ('expense', 12.34, 1, 1, 'lunch', '2026-07-01')",
            [],
        )
        .unwrap();
        insert_holding(&conn, "600000", "浦发银行", "cn", "stock");
        conn.execute(
            "UPDATE holdings SET current_price = 9.87 WHERE code = '600000'",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO price_history (holding_id, price, recorded_at) VALUES (1, 9.87, '2026-07-01 10:00:00')",
            [],
        )
        .unwrap();
        let before: (i64, f64, i64, String, i64, f64, i64, f64) = conn
            .query_row(
                "SELECT
                    (SELECT COUNT(*) FROM accounts),
                    (SELECT balance FROM accounts WHERE id = 1),
                    (SELECT COUNT(*) FROM transactions),
                    (SELECT note FROM transactions WHERE id = 1),
                    (SELECT COUNT(*) FROM holdings),
                    (SELECT current_price FROM holdings WHERE code = '600000'),
                    (SELECT COUNT(*) FROM price_history),
                    (SELECT price FROM price_history WHERE id = 1)",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                    ))
                },
            )
            .unwrap();
        let now = cn_datetime(2026, 7, 3, 10, 15, 0);
        let snapshots = vec![refresh_buy_snapshot("600000")];

        refresh_quant_signals_with_snapshots(&conn, &snapshots, now).unwrap();

        let after: (i64, f64, i64, String, i64, f64, i64, f64) = conn
            .query_row(
                "SELECT
                    (SELECT COUNT(*) FROM accounts),
                    (SELECT balance FROM accounts WHERE id = 1),
                    (SELECT COUNT(*) FROM transactions),
                    (SELECT note FROM transactions WHERE id = 1),
                    (SELECT COUNT(*) FROM holdings),
                    (SELECT current_price FROM holdings WHERE code = '600000'),
                    (SELECT COUNT(*) FROM price_history),
                    (SELECT price FROM price_history WHERE id = 1)",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                    ))
                },
            )
            .unwrap();

        assert_eq!(after, before);
        assert_eq!(signal_count(&conn), 1);
    }

    #[test]
    fn list_signals_filters_by_code_direction_and_limit() {
        let conn = test_conn();
        insert_signal(&conn, "000001", "Ping An", "buy", "2026-07-01 09:30:00");
        insert_signal(&conn, "000001", "Ping An", "sell", "2026-07-01 10:00:00");
        insert_signal(&conn, "000002", "Other", "buy", "2026-07-01 10:30:00");
        insert_signal(&conn, "000001", "Ping An", "buy", "2026-07-01 11:00:00");
        insert_signal(&conn, "000001", "Ping An", "buy", "2026-07-01 12:00:00");

        let signals = list_quant_signals_from_conn(
            &conn,
            Some(crate::models::QuantSignalFilter {
                code: Some("000001".to_string()),
                direction: Some("buy".to_string()),
                from: Some("2026-07-01 10:00:00".to_string()),
                to: Some("2026-07-01 12:00:00".to_string()),
                limit: Some(2),
            }),
        )
        .unwrap();

        assert_eq!(signals.len(), 2);
        assert_eq!(signals[0].code, "000001");
        assert_eq!(signals[0].direction, "buy");
        assert_eq!(signals[0].triggered_at, "2026-07-01 12:00:00");
        assert_eq!(signals[1].triggered_at, "2026-07-01 11:00:00");
    }

    #[test]
    fn list_signals_defaults_to_recent_100() {
        let conn = test_conn();
        for index in 0..105 {
            insert_signal(
                &conn,
                "000001",
                "Ping An",
                "buy",
                &format!("2026-07-01 10:00:{index:03}"),
            );
        }

        let signals = list_quant_signals_from_conn(&conn, None).unwrap();

        assert_eq!(signals.len(), 100);
        assert_eq!(signals[0].triggered_at, "2026-07-01 10:00:104");
        assert_eq!(signals[99].triggered_at, "2026-07-01 10:00:005");
    }

    #[test]
    fn list_signals_clamps_invalid_limits() {
        let conn = test_conn();
        for index in 0..600 {
            insert_signal(
                &conn,
                "000001",
                "Ping An",
                "buy",
                &format!("2026-07-01 10:00:{index:03}"),
            );
        }

        let negative_limit = list_quant_signals_from_conn(
            &conn,
            Some(crate::models::QuantSignalFilter {
                limit: Some(-1),
                ..Default::default()
            }),
        )
        .unwrap();
        let excessive_limit = list_quant_signals_from_conn(
            &conn,
            Some(crate::models::QuantSignalFilter {
                limit: Some(10_000),
                ..Default::default()
            }),
        )
        .unwrap();

        assert_eq!(negative_limit.len(), 100);
        assert_eq!(excessive_limit.len(), 500);
    }

    #[test]
    fn merge_targets_collapses_duplicate_holdings_and_watchlist() {
        let conn = test_conn();
        insert_holding(&conn, "000001", "Old Name", "cn", "stock");
        insert_holding(&conn, "000001", "", "cn", "stock");
        insert_holding(&conn, "000001", "Latest Name", "cn", "stock");
        insert_holding(&conn, "000002", "Fund Name", "cn", "fund");
        insert_watchlist(&conn, "000001", "Watch Name", "cn", false);
        insert_watchlist(&conn, "000003", "Watch Only", "cn", true);

        let targets = merge_quant_targets(&conn).unwrap();

        assert_eq!(targets.len(), 2);
        let held = targets
            .iter()
            .find(|target| target.code == "000001" && target.market == "cn")
            .unwrap();
        assert_eq!(held.name, "Latest Name");
        assert_eq!(held.source, "holding_watchlist");
        assert!(held.enabled);
        assert!(held.desktop_notification_enabled);
        assert_eq!(held.current_price, None);
        assert_eq!(held.quote_fetched_at, None);
        assert_eq!(held.trend_state, "insufficient_data");
        assert_eq!(held.output_state, "watch");
        assert_eq!(held.current_trigger_zone, None);
        assert_eq!(held.ma_short, None);
        assert_eq!(held.ma_long, None);
        assert!(held.grid_zones.is_empty());
        assert!(held.latest_signal.is_none());
        assert!(held.last_error.is_none());

        let watch_only = targets
            .iter()
            .find(|target| target.code == "000003" && target.market == "cn")
            .unwrap();
        assert_eq!(watch_only.name, "Watch Only");
        assert_eq!(watch_only.source, "watchlist");
        assert!(watch_only.enabled);
    }

    #[test]
    fn watchlist_disabled_default_stays_disabled_after_settings_insert() {
        let conn = test_conn();
        insert_watchlist(&conn, "000004", "Disabled Watch", "cn", false);

        let targets = merge_quant_targets(&conn).unwrap();

        assert_eq!(targets.len(), 1);
        assert!(!targets[0].enabled);

        let stored_enabled: i64 = conn
            .query_row(
                "SELECT enabled FROM quant_strategy_settings WHERE code = '000004' AND market = 'cn'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(stored_enabled, 0);
    }

    #[test]
    fn settings_enabled_overrides_watchlist_initial_enabled() {
        let conn = test_conn();
        insert_watchlist(&conn, "000005", "Disabled Watch", "cn", false);
        insert_settings(&conn, "000005", "cn", true, true);

        let targets = merge_quant_targets(&conn).unwrap();

        assert_eq!(targets.len(), 1);
        assert!(targets[0].enabled);
    }

    #[test]
    fn targets_default_to_auto_grid_when_settings_are_created() {
        let conn = test_conn();
        insert_watchlist(&conn, "000008", "Default Mode", "cn", true);

        let targets = merge_quant_targets(&conn).unwrap();

        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].strategy_mode, "auto_grid");
        let stored_mode: String = conn
            .query_row(
                "SELECT strategy_mode FROM quant_strategy_settings WHERE code = '000008' AND market = 'cn'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(stored_mode, "auto_grid");
    }

    #[test]
    fn merge_targets_reads_existing_strategy_mode() {
        let conn = test_conn();
        insert_watchlist(&conn, "000009", "Intraday Mode", "cn", true);
        insert_settings_with_mode(&conn, "000009", "cn", true, true, "intraday_t");

        let targets = merge_quant_targets(&conn).unwrap();

        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].strategy_mode, "intraday_t");
    }

    #[test]
    fn dashboard_read_only_targets_read_existing_strategy_mode() {
        let conn = test_conn();
        insert_watchlist(&conn, "000010", "Read Only Mode", "cn", true);
        insert_settings_with_mode(&conn, "000010", "cn", true, true, "intraday_t");

        let dashboard =
            build_quant_dashboard_with_snapshots(&conn, vec![], cn_datetime(2026, 1, 5, 10, 0, 0))
                .unwrap();

        assert_eq!(dashboard.targets.len(), 1);
        assert_eq!(dashboard.targets[0].strategy_mode, "intraday_t");
    }

    #[test]
    fn update_settings_persists_and_returns_strategy_mode() {
        let conn = test_conn();
        insert_watchlist(&conn, "000011", "Update Mode", "cn", true);

        let target = update_quant_strategy_settings_in_conn(
            &conn,
            "000011".to_string(),
            "cn".to_string(),
            QuantStrategySettingsUpdate {
                enabled: true,
                desktop_notification_enabled: true,
                strategy_mode: Some("intraday_t".to_string()),
            },
        )
        .unwrap();

        assert_eq!(target.strategy_mode, "intraday_t");
        let stored_mode: String = conn
            .query_row(
                "SELECT strategy_mode FROM quant_strategy_settings WHERE code = '000011' AND market = 'cn'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(stored_mode, "intraday_t");
    }

    #[test]
    fn update_settings_with_none_preserves_existing_strategy_mode() {
        let conn = test_conn();
        insert_watchlist(&conn, "000012", "Preserve Mode", "cn", true);
        insert_settings_with_mode(&conn, "000012", "cn", true, true, "intraday_t");

        let target = update_quant_strategy_settings_in_conn(
            &conn,
            "000012".to_string(),
            "cn".to_string(),
            QuantStrategySettingsUpdate {
                enabled: false,
                desktop_notification_enabled: false,
                strategy_mode: None,
            },
        )
        .unwrap();

        assert!(!target.enabled);
        assert!(!target.desktop_notification_enabled);
        assert_eq!(target.strategy_mode, "intraday_t");
        let stored_mode: String = conn
            .query_row(
                "SELECT strategy_mode FROM quant_strategy_settings WHERE code = '000012' AND market = 'cn'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(stored_mode, "intraday_t");
    }

    #[test]
    fn update_settings_with_none_defaults_new_settings_to_auto_grid() {
        let conn = test_conn();
        insert_watchlist(&conn, "000013", "Default Update Mode", "cn", true);

        let target = update_quant_strategy_settings_in_conn(
            &conn,
            "000013".to_string(),
            "cn".to_string(),
            QuantStrategySettingsUpdate {
                enabled: true,
                desktop_notification_enabled: true,
                strategy_mode: None,
            },
        )
        .unwrap();

        assert_eq!(target.strategy_mode, "auto_grid");
        let stored_mode: String = conn
            .query_row(
                "SELECT strategy_mode FROM quant_strategy_settings WHERE code = '000013' AND market = 'cn'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(stored_mode, "auto_grid");
    }

    #[test]
    fn update_settings_rejects_invalid_strategy_mode_without_writing() {
        let conn = test_conn();
        insert_watchlist(&conn, "000014", "Invalid Mode", "cn", true);

        let err = update_quant_strategy_settings_in_conn(
            &conn,
            "000014".to_string(),
            "cn".to_string(),
            QuantStrategySettingsUpdate {
                enabled: true,
                desktop_notification_enabled: true,
                strategy_mode: Some("invalid_mode".to_string()),
            },
        )
        .unwrap_err();

        assert!(
            err.contains("strategy_mode") || err.contains("策略"),
            "unexpected error: {err}"
        );
        let settings_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM quant_strategy_settings WHERE code = '000014' AND market = 'cn'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(settings_count, 0);
    }

    #[test]
    fn strategy_mode_settings_row_loads_intraday_numeric_defaults_and_values() {
        let conn = test_conn();

        let defaults = load_quant_strategy_settings_row(&conn, "000015", "cn").unwrap();
        assert_eq!(defaults.strategy_mode, "auto_grid");
        assert_eq!(defaults.intraday_lookback_days, 22);
        assert_eq!(defaults.intraday_high_time_min_count, 4);
        assert_eq!(defaults.intraday_low_time_min_count, 4);
        assert!((defaults.sell_t_position_threshold - 0.70).abs() < 1e-9);
        assert!((defaults.buyback_position_threshold - 0.30).abs() < 1e-9);

        conn.execute(
            "INSERT INTO quant_strategy_settings
                (code, market, strategy_mode, intraday_lookback_days,
                 intraday_high_time_min_count, intraday_low_time_min_count,
                 sell_t_position_threshold, buyback_position_threshold)
             VALUES ('000016', 'cn', 'intraday_t', 18, 3, 5, 0.75, 0.25)",
            [],
        )
        .unwrap();

        let custom = load_quant_strategy_settings_row(&conn, "000016", "cn").unwrap();
        assert_eq!(custom.strategy_mode, "intraday_t");
        assert_eq!(custom.intraday_lookback_days, 18);
        assert_eq!(custom.intraday_high_time_min_count, 3);
        assert_eq!(custom.intraday_low_time_min_count, 5);
        assert!((custom.sell_t_position_threshold - 0.75).abs() < 1e-9);
        assert!((custom.buyback_position_threshold - 0.25).abs() < 1e-9);
    }

    #[test]
    fn strategy_mode_settings_row_clamps_invalid_intraday_numeric_values() {
        let conn = test_conn();
        conn.execute(
            "INSERT INTO quant_strategy_settings
                (code, market, strategy_mode, intraday_lookback_days,
                 intraday_high_time_min_count, intraday_low_time_min_count,
                 sell_t_position_threshold, buyback_position_threshold)
             VALUES
                ('000017', 'cn', 'intraday_t', -5, -3, 0, -0.1, 1.5),
                ('000018', 'cn', 'intraday_t', 999, 999, 999, 0.8, 0.2)",
            [],
        )
        .unwrap();

        let invalid = load_quant_strategy_settings_row(&conn, "000017", "cn").unwrap();
        assert_eq!(invalid.intraday_lookback_days, 22);
        assert_eq!(invalid.intraday_high_time_min_count, 4);
        assert_eq!(invalid.intraday_low_time_min_count, 4);
        assert!((invalid.sell_t_position_threshold - 0.70).abs() < 1e-9);
        assert!((invalid.buyback_position_threshold - 0.30).abs() < 1e-9);

        let huge = load_quant_strategy_settings_row(&conn, "000018", "cn").unwrap();
        assert_eq!(huge.intraday_lookback_days, 60);
        assert_eq!(huge.intraday_high_time_min_count, 60);
        assert_eq!(huge.intraday_low_time_min_count, 60);
        assert!((huge.sell_t_position_threshold - 0.8).abs() < 1e-9);
        assert!((huge.buyback_position_threshold - 0.2).abs() < 1e-9);
    }

    #[test]
    fn desktop_notification_setting_does_not_disable_signal_generation() {
        let conn = test_conn();
        insert_watchlist(&conn, "000006", "Quiet Watch", "cn", true);

        let target = update_quant_strategy_settings_in_conn(
            &conn,
            "000006".to_string(),
            "cn".to_string(),
            QuantStrategySettingsUpdate {
                enabled: true,
                desktop_notification_enabled: false,
                strategy_mode: None,
            },
        )
        .unwrap();

        assert!(target.enabled);
        assert!(!target.desktop_notification_enabled);
    }

    #[test]
    fn update_settings_rejects_unsupported_market() {
        let conn = test_conn();
        insert_watchlist(&conn, "00700", "腾讯控股", "hk", true);

        let err = update_quant_strategy_settings_in_conn(
            &conn,
            "00700".to_string(),
            "hk".to_string(),
            QuantStrategySettingsUpdate {
                enabled: false,
                desktop_notification_enabled: false,
                strategy_mode: None,
            },
        )
        .unwrap_err();

        assert!(
            err.contains("unsupported market") || err.contains("市场"),
            "unexpected error: {err}"
        );
        let settings_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM quant_strategy_settings", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(settings_count, 0);
    }

    #[test]
    fn update_settings_for_missing_target_does_not_insert_orphan_settings() {
        let conn = test_conn();
        insert_holding(&conn, "000001", "Held Name", "cn", "stock");

        let result = update_quant_strategy_settings_in_conn(
            &conn,
            "600000".to_string(),
            "cn".to_string(),
            QuantStrategySettingsUpdate {
                enabled: true,
                desktop_notification_enabled: true,
                strategy_mode: None,
            },
        );

        assert!(result.is_err());
        let settings_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM quant_strategy_settings", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(settings_count, 0);
    }

    #[test]
    fn deleting_watchlist_keeps_held_target_available() {
        let conn = test_conn();
        insert_holding(&conn, "000007", "Held Name", "cn", "stock");
        let watchlist = add_watchlist_to_conn(
            &conn,
            NewQuantWatchlistItem {
                code: "000007".to_string(),
                name: Some("Watch Name".to_string()),
                market: Some("cn".to_string()),
                enabled: Some(false),
            },
        )
        .unwrap();

        delete_watchlist_from_conn(&conn, watchlist.id).unwrap();
        let targets = merge_quant_targets(&conn).unwrap();

        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].code, "000007");
        assert_eq!(targets[0].name, "Held Name");
        assert_eq!(targets[0].source, "holding");
        assert!(targets[0].enabled);
    }
}
