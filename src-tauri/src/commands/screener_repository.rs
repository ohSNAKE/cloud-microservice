use crate::db::prune_derived_screener_data;
use crate::models::{
    ScreenerDashboard, ScreenerFailedAttempt, ScreenerFailureSummary, ScreenerResult, ScreenerRun,
};
use rusqlite::{params, Connection, OptionalExtension};

#[derive(Debug, Clone)]
pub struct PendingScreenerRun {
    pub status: String,
    pub started_at: String,
    pub completed_at: String,
    pub candidate_count: i64,
    pub match_count: i64,
    pub skipped_count: i64,
    pub failure_code: Option<String>,
    pub failure_message: Option<String>,
}

pub fn persist_completed_run(
    conn: &Connection,
    run: PendingScreenerRun,
    results: Vec<ScreenerResult>,
) -> Result<ScreenerDashboard, String> {
    let tx = conn
        .unchecked_transaction()
        .map_err(|error| error.to_string())?;
    tx.execute(
        "INSERT INTO screener_runs
         (status, started_at, completed_at, candidate_count, match_count, skipped_count, failure_code, failure_message)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            run.status,
            run.started_at,
            run.completed_at,
            run.candidate_count,
            results.len() as i64,
            run.skipped_count,
            run.failure_code,
            run.failure_message,
        ],
    )
    .map_err(|error| error.to_string())?;
    let run_id = tx.last_insert_rowid();
    for result in results {
        tx.execute(
            "INSERT INTO screener_results
             (run_id, code, exchange, name, market_cap_cny, cash_dividend_per_share, dividend_yield,
              current_price, price_observed_at, daily_lower_band, daily_distance, daily_kline_completed_at,
              weekly_lower_band, weekly_distance, weekly_kline_completed_at, matched_periods_json,
              source_metadata, fundamental_observed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
            params![
                run_id,
                result.code,
                result.exchange,
                result.name,
                result.market_cap_cny,
                result.cash_dividend_per_share,
                result.dividend_yield,
                result.current_price,
                result.price_observed_at,
                result.daily_lower_band,
                result.daily_distance,
                result.daily_kline_completed_at,
                result.weekly_lower_band,
                result.weekly_distance,
                result.weekly_kline_completed_at,
                serde_json::to_string(&result.matched_periods).map_err(|error| error.to_string())?,
                result.source_metadata,
                result.fundamental_observed_at,
            ],
        )
        .map_err(|error| error.to_string())?;
    }
    tx.commit().map_err(|error| error.to_string())?;
    prune_derived_screener_data(conn).map_err(|error| error.to_string())?;
    read_screener_dashboard(conn)
}

pub fn persist_failed_attempt(
    conn: &Connection,
    failure_code: &str,
    failure_message: &str,
) -> Result<ScreenerDashboard, String> {
    conn.execute(
        "INSERT INTO screener_runs
         (status, started_at, completed_at, candidate_count, match_count, skipped_count, failure_code, failure_message)
         VALUES ('failed', datetime('now'), datetime('now'), 0, 0, 0, ?1, ?2)",
        params![failure_code, failure_message],
    )
    .map_err(|error| error.to_string())?;
    prune_derived_screener_data(conn).map_err(|error| error.to_string())?;
    read_screener_dashboard(conn)
}

/// Load cached actual controllers keyed by (code, exchange), preferring the most
/// recent valuation_date so a stock's controller is reused across days.
pub fn load_actual_controller_cache(
    conn: &Connection,
) -> Result<std::collections::HashMap<(String, String), String>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT code, exchange, actual_controller FROM screener_actual_controllers ac
             WHERE valuation_date = (
               SELECT MAX(valuation_date) FROM screener_actual_controllers
               WHERE code = ac.code AND exchange = ac.exchange
             )",
        )
        .map_err(|error| error.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                (row.get::<_, String>(0)?, row.get::<_, String>(1)?),
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|error| error.to_string())?;
    let mut cache = std::collections::HashMap::new();
    for row in rows {
        let ((code, exchange), controller) = row.map_err(|error| error.to_string())?;
        cache.insert((code, exchange), controller);
    }
    Ok(cache)
}

pub fn save_actual_controllers(
    conn: &Connection,
    valuation_date: &str,
    controllers: &[(String, String, String)],
) -> Result<(), String> {
    let tx = conn
        .unchecked_transaction()
        .map_err(|error| error.to_string())?;
    let observed_at = chrono::Utc::now().to_rfc3339();
    for (code, exchange, controller) in controllers {
        if controller.trim().is_empty() {
            continue;
        }
        tx.execute(
            "INSERT OR REPLACE INTO screener_actual_controllers
             (code, exchange, valuation_date, actual_controller, observed_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![code, exchange, valuation_date, controller, observed_at],
        )
        .map_err(|error| error.to_string())?;
    }
    tx.commit().map_err(|error| error.to_string())?;
    Ok(())
}

pub fn read_screener_dashboard(conn: &Connection) -> Result<ScreenerDashboard, String> {
    let displayed_run = conn
        .query_row(
            "SELECT id, status, started_at, completed_at, candidate_count, match_count,
                    skipped_count, failure_code, failure_message
             FROM screener_runs WHERE status IN ('success', 'partial') ORDER BY id DESC LIMIT 1",
            [],
            map_run,
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let latest_failed_run = conn
        .query_row(
            "SELECT id, status, started_at, completed_at, candidate_count, match_count,
                    skipped_count, failure_code, failure_message
             FROM screener_runs
             WHERE status = 'failed'
               AND id > (SELECT COALESCE(MAX(id), 0) FROM screener_runs WHERE status IN ('success', 'partial'))
             ORDER BY id DESC LIMIT 1",
            [],
            map_run,
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let results = match &displayed_run {
        Some(run) => read_results(conn, run.id)?,
        None => Vec::new(),
    };
    let is_stale = match (&displayed_run, &latest_failed_run) {
        (Some(displayed), Some(failed)) => failed.id > displayed.id,
        _ => false,
    };
    let latest_failed_attempt = latest_failed_run.map(|run| ScreenerFailedAttempt {
        started_at: run.started_at,
        completed_at: run.completed_at,
        failure_summary: run.failure_summary,
    });

    Ok(ScreenerDashboard {
        displayed_run,
        latest_failed_attempt,
        is_stale,
        results,
    })
}

fn map_run(row: &rusqlite::Row<'_>) -> rusqlite::Result<ScreenerRun> {
    let failure_code: Option<String> = row.get(7)?;
    let failure_message: Option<String> = row.get(8)?;
    let skipped_count = row.get(6)?;
    Ok(ScreenerRun {
        id: row.get(0)?,
        status: row.get(1)?,
        started_at: row.get(2)?,
        completed_at: row.get(3)?,
        candidate_count: row.get(4)?,
        match_count: row.get(5)?,
        skipped_count,
        failure_summary: failure_code.map(|code| ScreenerFailureSummary {
            code,
            message: failure_message.unwrap_or_default(),
            skipped_count,
        }),
    })
}

fn read_results(conn: &Connection, run_id: i64) -> Result<Vec<ScreenerResult>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT code, exchange, name, market_cap_cny, cash_dividend_per_share, dividend_yield,
                    current_price, price_observed_at, daily_lower_band, daily_distance,
                    daily_kline_completed_at, weekly_lower_band, weekly_distance, weekly_kline_completed_at,
                    matched_periods_json, source_metadata, fundamental_observed_at
             FROM screener_results WHERE run_id = ?1 ORDER BY code, exchange",
        )
        .map_err(|error| error.to_string())?;
    let results = stmt
        .query_map([run_id], |row| {
            let matched_periods_json: String = row.get(14)?;
            let matched_periods = serde_json::from_str::<Vec<String>>(&matched_periods_json)
                .map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        14,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?;
            Ok(ScreenerResult {
                code: row.get(0)?,
                exchange: row.get(1)?,
                name: row.get(2)?,
                market_cap_cny: row.get(3)?,
                cash_dividend_per_share: row.get(4)?,
                dividend_yield: row.get(5)?,
                current_price: row.get(6)?,
                price_observed_at: row.get(7)?,
                daily_lower_band: row.get(8)?,
                daily_distance: row.get(9)?,
                daily_kline_completed_at: row.get(10)?,
                weekly_lower_band: row.get(11)?,
                weekly_distance: row.get(12)?,
                weekly_kline_completed_at: row.get(13)?,
                matched_periods,
                source_metadata: row.get(15)?,
                fundamental_observed_at: row.get(16)?,
            })
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::ensure_screener_schema;
    use crate::models::ScreenerResult;
    use rusqlite::Connection;

    fn setup_screener_schema() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        ensure_screener_schema(&conn).unwrap();
        conn
    }

    fn run(status: &str, completed_at: &str) -> PendingScreenerRun {
        PendingScreenerRun {
            status: status.into(),
            started_at: completed_at.into(),
            completed_at: completed_at.into(),
            candidate_count: 1,
            match_count: 1,
            skipped_count: if status == "partial" { 1 } else { 0 },
            failure_code: None,
            failure_message: None,
        }
    }

    fn result(code: &str) -> ScreenerResult {
        ScreenerResult {
            code: code.into(),
            exchange: "sh".into(),
            name: "测试".into(),
            market_cap_cny: 50_000_000_000.0,
            cash_dividend_per_share: 1.0,
            dividend_yield: 0.05,
            current_price: 20.0,
            price_observed_at: "2026-07-21T01:00:00Z".into(),
            daily_lower_band: Some(19.8),
            daily_distance: Some(0.01),
            daily_kline_completed_at: Some("2026-07-21T07:00:00Z".into()),
            weekly_lower_band: None,
            weekly_distance: None,
            weekly_kline_completed_at: None,
            matched_periods: vec!["day".into()],
            source_metadata: "{}".into(),
            fundamental_observed_at: "2026-07-21T01:00:00Z".into(),
        }
    }

    #[test]
    fn completed_partial_run_replaces_displayed_results_atomically() {
        let conn = setup_screener_schema();
        persist_completed_run(
            &conn,
            run("partial", "2026-07-21T01:00:00Z"),
            vec![result("600001")],
        )
        .unwrap();
        let dashboard = read_screener_dashboard(&conn).unwrap();
        assert_eq!(dashboard.displayed_run.unwrap().status, "partial");
        assert_eq!(dashboard.results.len(), 1);
    }

    #[test]
    fn controller_cache_round_trips_by_code_and_exchange() {
        let conn = setup_screener_schema();
        save_actual_controllers(
            &conn,
            "2026-07-21",
            &[
                (
                    "600001".to_string(),
                    "sh".to_string(),
                    "中国移动通信集团有限公司".to_string(),
                ),
                (
                    "000002".to_string(),
                    "sz".to_string(),
                    "民营企业".to_string(),
                ),
            ],
        )
        .unwrap();

        let cache = load_actual_controller_cache(&conn).unwrap();

        assert_eq!(
            cache.get(&("600001".to_string(), "sh".to_string())),
            Some(&"中国移动通信集团有限公司".to_string())
        );
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn successful_run_after_failure_clears_failed_attempt_and_is_not_stale() {
        let conn = setup_screener_schema();
        persist_failed_attempt(&conn, "provider_unavailable", "选股刷新失败：dash").unwrap();
        persist_completed_run(
            &conn,
            run("success", "2026-07-22T01:00:00Z"),
            vec![result("600001")],
        )
        .unwrap();
        let dashboard = read_screener_dashboard(&conn).unwrap();
        assert!(!dashboard.is_stale);
        assert!(dashboard.latest_failed_attempt.is_none());
    }

    #[test]
    fn failed_attempt_retains_last_completed_results_and_marks_dashboard_stale() {
        let conn = setup_screener_schema();
        persist_completed_run(
            &conn,
            run("success", "2026-07-21T01:00:00Z"),
            vec![result("600001")],
        )
        .unwrap();
        persist_failed_attempt(&conn, "provider_unavailable", "timeout").unwrap();
        let dashboard = read_screener_dashboard(&conn).unwrap();
        assert!(dashboard.is_stale);
        assert_eq!(dashboard.results[0].code, "600001");
        assert_eq!(
            dashboard
                .latest_failed_attempt
                .unwrap()
                .failure_summary
                .unwrap()
                .code,
            "provider_unavailable"
        );
    }
}
