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
        CREATE INDEX IF NOT EXISTS idx_quant_signals_code_time ON quant_signals(code, triggered_at);
        CREATE INDEX IF NOT EXISTS idx_quant_signals_dedupe_time ON quant_signals(dedupe_key, triggered_at);
        ",
    )?;

    conn.execute(
        "UPDATE settings SET value = 'https://v2.pincc.ai/v1'
         WHERE key = 'ai_api_base' AND value = 'https://api.openai.com/v1'",
        [],
    )
    .ok();

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
