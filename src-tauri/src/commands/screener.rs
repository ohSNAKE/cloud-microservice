use crate::models::{ScreenerDashboard, ScreenerRefreshSchedule, ScreenerWatchlistAddResult};
use crate::services::screener_calendar::schedule_at;
use crate::{commands::screener_repository::read_screener_dashboard, AppState};
use chrono::Utc;
use rusqlite::{params, Connection};
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
pub fn refresh_screener(state: State<AppState>) -> Result<ScreenerDashboard, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    read_screener_dashboard(&conn)
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

        let result = add_screener_result_to_watchlist_in_conn(&conn, "600001", "sh", "测试").unwrap();

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
        assert!(add_screener_result_to_watchlist_in_conn(&conn, "600001", "sh", "上证").unwrap().added);
        assert!(add_screener_result_to_watchlist_in_conn(&conn, "000001", "sz", "深证").unwrap().added);
        assert!(add_screener_result_to_watchlist_in_conn(&conn, "830001", "bj", "北证").unwrap().added);
        assert!(add_screener_result_to_watchlist_in_conn(&conn, "00700", "hk", "腾讯").is_err());
    }
}
