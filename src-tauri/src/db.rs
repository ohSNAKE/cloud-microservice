use chrono::Local;
use rusqlite::{params, Connection};
use std::path::PathBuf;
use tauri::Manager;

pub fn init_db(app_handle: &tauri::AppHandle) -> Result<Connection, rusqlite::Error> {
    let mut db_path = app_handle
        .path()
        .app_data_dir()
        .expect("failed to resolve app data dir");
    std::fs::create_dir_all(&db_path).ok();
    db_path.push("finance.db");

    let conn = Connection::open(db_path)?;
    conn.execute_batch(
        "
        PRAGMA journal_mode = WAL;
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS accounts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            type TEXT NOT NULL DEFAULT 'cash',
            balance REAL NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
        );

        CREATE TABLE IF NOT EXISTS categories (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            type TEXT NOT NULL,
            icon TEXT DEFAULT 'pushpin'
        );

        CREATE TABLE IF NOT EXISTS transactions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            type TEXT NOT NULL,
            amount REAL NOT NULL,
            category_id INTEGER,
            account_id INTEGER,
            transfer_to_account_id INTEGER,
            note TEXT DEFAULT '',
            transaction_date TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            FOREIGN KEY (category_id) REFERENCES categories(id),
            FOREIGN KEY (account_id) REFERENCES accounts(id),
            FOREIGN KEY (transfer_to_account_id) REFERENCES accounts(id)
        );

        CREATE TABLE IF NOT EXISTS holdings (
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

        CREATE TABLE IF NOT EXISTS price_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            holding_id INTEGER NOT NULL,
            price REAL NOT NULL,
            recorded_at TEXT NOT NULL,
            FOREIGN KEY (holding_id) REFERENCES holdings(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS budgets (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            category_id INTEGER NOT NULL,
            month TEXT NOT NULL,
            amount REAL NOT NULL,
            UNIQUE(category_id, month),
            FOREIGN KEY (category_id) REFERENCES categories(id)
        );

        CREATE TABLE IF NOT EXISTS recurring_rules (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            type TEXT NOT NULL,
            amount REAL NOT NULL,
            category_id INTEGER,
            account_id INTEGER,
            note TEXT DEFAULT '',
            day_of_month INTEGER NOT NULL DEFAULT 1,
            enabled INTEGER NOT NULL DEFAULT 1,
            last_run_month TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            FOREIGN KEY (category_id) REFERENCES categories(id),
            FOREIGN KEY (account_id) REFERENCES accounts(id)
        );

        CREATE TABLE IF NOT EXISTS quant_watchlist (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            code TEXT NOT NULL,
            name TEXT NOT NULL,
            market TEXT NOT NULL DEFAULT 'cn',
            enabled INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            UNIQUE(code, market)
        );

        CREATE TABLE IF NOT EXISTS quant_strategy_settings (
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

        CREATE TABLE IF NOT EXISTS quant_signals (
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

        CREATE TABLE IF NOT EXISTS quant_intraday_t_state (
            code TEXT NOT NULL,
            market TEXT NOT NULL,
            trading_date TEXT NOT NULL,
            sold_at TEXT NOT NULL,
            PRIMARY KEY (code, market, trading_date)
        );

        CREATE INDEX IF NOT EXISTS idx_transactions_date ON transactions(transaction_date);
        CREATE INDEX IF NOT EXISTS idx_price_history_holding ON price_history(holding_id, recorded_at);
        CREATE INDEX IF NOT EXISTS idx_quant_signals_code_time ON quant_signals(code, triggered_at);
        CREATE INDEX IF NOT EXISTS idx_quant_signals_dedupe_time ON quant_signals(dedupe_key, triggered_at);
        ",
    )?;

    migrate(&conn)?;
    seed_defaults(&conn)?;
    Ok(conn)
}

fn ensure_column(
    conn: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<(), rusqlite::Error> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let has_column = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .filter_map(Result::ok)
        .any(|name| name == column);

    if !has_column {
        conn.execute(&format!("ALTER TABLE {table} ADD COLUMN {definition}"), [])?;
    }

    Ok(())
}

fn migrate(conn: &Connection) -> Result<(), rusqlite::Error> {
    let has_transfer_col: bool = conn
        .prepare("PRAGMA table_info(transactions)")?
        .query_map([], |row| {
            let name: String = row.get(1)?;
            Ok(name == "transfer_to_account_id")
        })?
        .filter_map(Result::ok)
        .any(|v| v);

    if !has_transfer_col {
        conn.execute(
            "ALTER TABLE transactions ADD COLUMN transfer_to_account_id INTEGER",
            [],
        )?;
    }

    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS budgets (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            category_id INTEGER NOT NULL,
            month TEXT NOT NULL,
            amount REAL NOT NULL,
            UNIQUE(category_id, month),
            FOREIGN KEY (category_id) REFERENCES categories(id)
        );
        CREATE TABLE IF NOT EXISTS recurring_rules (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            type TEXT NOT NULL,
            amount REAL NOT NULL,
            category_id INTEGER,
            account_id INTEGER,
            note TEXT DEFAULT '',
            day_of_month INTEGER NOT NULL DEFAULT 1,
            enabled INTEGER NOT NULL DEFAULT 1,
            last_run_month TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            FOREIGN KEY (category_id) REFERENCES categories(id),
            FOREIGN KEY (account_id) REFERENCES accounts(id)
        );
        CREATE TABLE IF NOT EXISTS quant_watchlist (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            code TEXT NOT NULL,
            name TEXT NOT NULL,
            market TEXT NOT NULL DEFAULT 'cn',
            enabled INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            UNIQUE(code, market)
        );
        CREATE TABLE IF NOT EXISTS quant_strategy_settings (
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
        CREATE TABLE IF NOT EXISTS quant_signals (
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
        CREATE TABLE IF NOT EXISTS quant_intraday_t_state (
            code TEXT NOT NULL,
            market TEXT NOT NULL,
            trading_date TEXT NOT NULL,
            sold_at TEXT NOT NULL,
            PRIMARY KEY (code, market, trading_date)
        );
        CREATE INDEX IF NOT EXISTS idx_quant_signals_code_time ON quant_signals(code, triggered_at);
        CREATE INDEX IF NOT EXISTS idx_quant_signals_dedupe_time ON quant_signals(dedupe_key, triggered_at);
        ",
    )?;

    for (column, definition) in [
        (
            "strategy_mode",
            "strategy_mode TEXT NOT NULL DEFAULT 'auto_grid'",
        ),
        (
            "intraday_lookback_days",
            "intraday_lookback_days INTEGER NOT NULL DEFAULT 22",
        ),
        (
            "intraday_high_time_min_count",
            "intraday_high_time_min_count INTEGER NOT NULL DEFAULT 4",
        ),
        (
            "intraday_low_time_min_count",
            "intraday_low_time_min_count INTEGER NOT NULL DEFAULT 4",
        ),
        (
            "sell_t_position_threshold",
            "sell_t_position_threshold REAL NOT NULL DEFAULT 0.70",
        ),
        (
            "buyback_position_threshold",
            "buyback_position_threshold REAL NOT NULL DEFAULT 0.30",
        ),
    ] {
        ensure_column(conn, "quant_strategy_settings", column, definition)?;
    }

    conn.execute(
        "UPDATE settings SET value = 'https://v2.pincc.ai/v1'
         WHERE key = 'ai_api_base' AND value = 'https://api.openai.com/v1'",
        [],
    )
    .ok();

    ensure_screener_schema(conn)?;

    Ok(())
}

pub fn ensure_screener_schema(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS screener_fundamentals (
          code TEXT NOT NULL, exchange TEXT NOT NULL, valuation_date TEXT NOT NULL,
          dividend_year INTEGER NOT NULL, name TEXT NOT NULL, market_cap_cny REAL NOT NULL,
          actual_controller TEXT NOT NULL, controller_classification TEXT NOT NULL,
          cash_dividend_per_share REAL NOT NULL, source_metadata TEXT NOT NULL,
          observed_at TEXT NOT NULL,
          PRIMARY KEY (code, exchange, valuation_date, dividend_year)
        );
        CREATE TABLE IF NOT EXISTS screener_controller_registry (
          valuation_date TEXT NOT NULL, legal_name TEXT NOT NULL, normalized_name TEXT NOT NULL,
          aliases_json TEXT NOT NULL, source_url TEXT NOT NULL, source_date TEXT NOT NULL,
          PRIMARY KEY (valuation_date, normalized_name)
        );
        CREATE TABLE IF NOT EXISTS screener_quotes (
          code TEXT NOT NULL, exchange TEXT NOT NULL, valuation_date TEXT NOT NULL,
          price REAL NOT NULL, observed_at TEXT NOT NULL,
          PRIMARY KEY (code, exchange, valuation_date)
        );
        CREATE TABLE IF NOT EXISTS screener_runs (
          id INTEGER PRIMARY KEY AUTOINCREMENT, status TEXT NOT NULL, started_at TEXT NOT NULL,
          completed_at TEXT NOT NULL, candidate_count INTEGER NOT NULL, match_count INTEGER NOT NULL,
          skipped_count INTEGER NOT NULL, failure_code TEXT, failure_message TEXT
        );
        CREATE TABLE IF NOT EXISTS screener_results (
          run_id INTEGER NOT NULL, code TEXT NOT NULL, exchange TEXT NOT NULL, name TEXT NOT NULL,
          market_cap_cny REAL NOT NULL, cash_dividend_per_share REAL NOT NULL, dividend_yield REAL NOT NULL,
          current_price REAL NOT NULL, price_observed_at TEXT NOT NULL,
          daily_lower_band REAL, daily_distance REAL, daily_kline_completed_at TEXT,
          weekly_lower_band REAL, weekly_distance REAL, weekly_kline_completed_at TEXT,
          matched_periods_json TEXT NOT NULL, source_metadata TEXT NOT NULL, fundamental_observed_at TEXT NOT NULL,
          PRIMARY KEY (run_id, code, exchange), FOREIGN KEY (run_id) REFERENCES screener_runs(id)
        );
        CREATE TABLE IF NOT EXISTS screener_state (key TEXT PRIMARY KEY, value TEXT NOT NULL);
        CREATE INDEX IF NOT EXISTS idx_screener_results_run ON screener_results(run_id);
        ",
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO screener_state (key, value) VALUES ('generation', '0')",
        [],
    )?;
    Ok(())
}

#[allow(dead_code)]
pub fn prune_derived_screener_data(conn: &Connection) -> Result<(), rusqlite::Error> {
    for table in [
        "screener_fundamentals",
        "screener_controller_registry",
        "screener_quotes",
    ] {
        conn.execute(
            &format!(
                "DELETE FROM {table}
                 WHERE valuation_date NOT IN (
                   SELECT valuation_date FROM (
                     SELECT DISTINCT valuation_date FROM {table} ORDER BY valuation_date DESC LIMIT 7
                   )
                 )"
            ),
            [],
        )?;
    }

    conn.execute(
        "DELETE FROM screener_results
         WHERE run_id NOT IN (SELECT id FROM screener_runs ORDER BY id DESC LIMIT 30)",
        [],
    )?;
    conn.execute(
        "DELETE FROM screener_runs
         WHERE id NOT IN (SELECT id FROM screener_runs ORDER BY id DESC LIMIT 30)",
        [],
    )?;
    Ok(())
}

fn seed_defaults(conn: &Connection) -> Result<(), rusqlite::Error> {
    let account_count: i64 =
        conn.query_row("SELECT COUNT(*) FROM accounts", [], |row| row.get(0))?;
    if account_count == 0 {
        for (name, kind) in [
            ("现金", "cash"),
            ("银行卡", "bank"),
            ("支付宝", "alipay"),
            ("微信", "wechat"),
            ("证券账户", "broker"),
        ] {
            conn.execute(
                "INSERT INTO accounts (name, type, balance) VALUES (?1, ?2, 0)",
                params![name, kind],
            )?;
        }
    }

    let category_count: i64 =
        conn.query_row("SELECT COUNT(*) FROM categories", [], |row| row.get(0))?;
    if category_count == 0 {
        let categories = [
            ("餐饮", "expense", "dining"),
            ("交通", "expense", "transport"),
            ("购物", "expense", "shopping"),
            ("住房", "expense", "housing"),
            ("娱乐", "expense", "entertainment"),
            ("医疗", "expense", "medical"),
            ("教育", "expense", "education"),
            ("其他支出", "expense", "pushpin"),
            ("工资", "income", "salary"),
            ("奖金", "income", "gift"),
            ("理财收益", "income", "investment"),
            ("其他收入", "income", "cash-income"),
        ];
        for (name, kind, icon) in categories {
            conn.execute(
                "INSERT INTO categories (name, type, icon) VALUES (?1, ?2, ?3)",
                params![name, kind, icon],
            )?;
        }
    }

    let settings_count: i64 =
        conn.query_row("SELECT COUNT(*) FROM settings", [], |row| row.get(0))?;
    if settings_count == 0 {
        for (key, value) in [
            ("quote_update_interval", "30"),
            ("quote_update_enabled", "true"),
            ("refresh_on_startup", "true"),
            ("currency", "CNY"),
            ("ai_enabled", "true"),
            ("ai_api_key", ""),
            ("ai_api_base", "https://v2.pincc.ai/v1"),
            ("ai_model", "gpt-4o-mini"),
        ] {
            conn.execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)",
                params![key, value],
            )?;
        }
    } else {
        conn.execute(
            "INSERT OR IGNORE INTO settings (key, value) VALUES ('refresh_on_startup', 'true')",
            [],
        )?;
    }

    for (key, value) in [
        ("ai_enabled", "true"),
        ("ai_api_key", ""),
        ("ai_api_base", "https://v2.pincc.ai/v1"),
        ("ai_model", "gpt-4o-mini"),
    ] {
        conn.execute(
            "INSERT OR IGNORE INTO settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
    }

    Ok(())
}

pub fn get_setting(conn: &Connection, key: &str) -> Option<String> {
    conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        params![key],
        |row| row.get(0),
    )
    .ok()
}

pub fn db_path_hint(app_handle: &tauri::AppHandle) -> PathBuf {
    let mut path = app_handle
        .path()
        .app_data_dir()
        .expect("failed to resolve app data dir");
    path.push("finance.db");
    path
}

pub fn now_local() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

pub fn today() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

pub fn current_month() -> String {
    Local::now().format("%Y-%m").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn test_connection_after_migration() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "
            PRAGMA foreign_keys = ON;
            CREATE TABLE transactions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                type TEXT NOT NULL,
                amount REAL NOT NULL,
                transaction_date TEXT NOT NULL
            );
            CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
            ",
        )
        .unwrap();
        migrate(&conn).unwrap();
        conn
    }

    fn table_exists(conn: &Connection, table: &str) -> bool {
        conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [table],
            |row| row.get::<_, i64>(0),
        )
        .unwrap()
            == 1
    }

    fn index_exists(conn: &Connection, index: &str) -> bool {
        conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name = ?1",
            [index],
            |row| row.get::<_, i64>(0),
        )
        .unwrap()
            == 1
    }

    #[test]
    fn migration_creates_screener_cache_and_result_tables() {
        let conn = test_connection_after_migration();
        for table in [
            "screener_fundamentals",
            "screener_controller_registry",
            "screener_quotes",
            "screener_runs",
            "screener_results",
            "screener_state",
        ] {
            assert!(table_exists(&conn, table), "missing table {table}");
        }
        assert!(index_exists(&conn, "idx_screener_results_run"));
    }

    #[test]
    fn result_rows_require_a_run() {
        let conn = test_connection_after_migration();
        let error = conn
            .execute(
                "INSERT INTO screener_results
                 (run_id, code, exchange, name, market_cap_cny, cash_dividend_per_share,
                  dividend_yield, current_price, price_observed_at, matched_periods_json,
                  source_metadata, fundamental_observed_at)
                 VALUES (999, '600001', 'sh', '测试', 50000000000, 1.0, 0.05, 10.0,
                         '2026-07-21T01:00:00Z', '[\"day\"]', '{}', '2026-07-21T01:00:00Z')",
                [],
            )
            .unwrap_err();
        assert!(error.to_string().contains("FOREIGN KEY"));
    }

    #[test]
    fn prune_derived_screener_data_retains_recent_dates_and_runs() {
        let conn = test_connection_after_migration();
        for day in 1..=8 {
            conn.execute(
                "INSERT INTO screener_fundamentals
                 (code, exchange, valuation_date, dividend_year, name, market_cap_cny,
                  actual_controller, controller_classification, cash_dividend_per_share,
                  source_metadata, observed_at)
                 VALUES ('600001', 'sh', ?1, 2025, '测试', 50000000000, '国务院',
                         'central_soe', 1.0, '{}', '2026-07-21T01:00:00Z')",
                [format!("2026-07-{day:02}")],
            )
            .unwrap();
        }
        for run in 1..=31 {
            conn.execute(
                "INSERT INTO screener_runs
                 (status, started_at, completed_at, candidate_count, match_count, skipped_count)
                 VALUES ('success', ?1, ?1, 0, 0, 0)",
                [format!("2026-07-21T00:{run:02}:00Z")],
            )
            .unwrap();
        }

        prune_derived_screener_data(&conn).unwrap();

        let fundamental_dates: i64 = conn
            .query_row(
                "SELECT COUNT(DISTINCT valuation_date) FROM screener_fundamentals",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let runs: i64 = conn
            .query_row("SELECT COUNT(*) FROM screener_runs", [], |row| row.get(0))
            .unwrap();
        assert_eq!(fundamental_dates, 7);
        assert_eq!(runs, 30);
    }
}
