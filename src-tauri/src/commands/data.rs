use crate::db::{db_path_hint, now_local};
use crate::models::{
    Account, Category, ExportPayload, HoldingRow, QuantSignalRow, QuantStrategySettingsRow,
    QuantWatchlistRow, TransactionRow,
};
use crate::AppState;
use rusqlite::{params, Connection};
use tauri::State;

#[tauri::command]
pub fn get_db_path(app: tauri::AppHandle) -> Result<String, String> {
    Ok(db_path_hint(&app).to_string_lossy().to_string())
}

#[tauri::command]
pub fn export_data(state: State<AppState>) -> Result<String, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let payload = export_payload_from_conn(&conn)?;

    serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())
}

fn export_payload_from_conn(conn: &Connection) -> Result<ExportPayload, String> {
    let mut account_stmt = conn
        .prepare("SELECT id, name, type, balance, created_at FROM accounts ORDER BY id")
        .map_err(|e| e.to_string())?;
    let accounts: Vec<Account> = account_stmt
        .query_map([], |row| {
            Ok(Account {
                id: row.get(0)?,
                name: row.get(1)?,
                r#type: row.get(2)?,
                balance: row.get(3)?,
                created_at: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut category_stmt = conn
        .prepare("SELECT id, name, type, icon FROM categories ORDER BY id")
        .map_err(|e| e.to_string())?;
    let categories: Vec<Category> = category_stmt
        .query_map([], |row| {
            Ok(Category {
                id: row.get(0)?,
                name: row.get(1)?,
                r#type: row.get(2)?,
                icon: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut tx_stmt = conn
        .prepare(
            "SELECT type, amount, category_id, account_id, transfer_to_account_id, note, transaction_date
             FROM transactions ORDER BY transaction_date, id",
        )
        .map_err(|e| e.to_string())?;
    let transactions: Vec<TransactionRow> = tx_stmt
        .query_map([], |row| {
            Ok(TransactionRow {
                r#type: row.get(0)?,
                amount: row.get(1)?,
                category_id: row.get(2)?,
                account_id: row.get(3)?,
                transfer_to_account_id: row.get(4)?,
                note: row.get(5)?,
                transaction_date: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut holding_stmt = conn
        .prepare(
            "SELECT code, name, type, quantity, cost_price, current_price, market FROM holdings ORDER BY id",
        )
        .map_err(|e| e.to_string())?;
    let holdings: Vec<HoldingRow> = holding_stmt
        .query_map([], |row| {
            Ok(HoldingRow {
                code: row.get(0)?,
                name: row.get(1)?,
                r#type: row.get(2)?,
                quantity: row.get(3)?,
                cost_price: row.get(4)?,
                current_price: row.get(5)?,
                market: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut settings_stmt = conn
        .prepare("SELECT key, value FROM settings")
        .map_err(|e| e.to_string())?;
    let settings: Vec<(String, String)> = settings_stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut quant_watchlist_stmt = conn
        .prepare(
            "SELECT id, code, name, market, enabled, created_at, updated_at
             FROM quant_watchlist ORDER BY id",
        )
        .map_err(|e| e.to_string())?;
    let quant_watchlist: Vec<QuantWatchlistRow> = quant_watchlist_stmt
        .query_map([], |row| {
            let enabled: i64 = row.get(4)?;
            Ok(QuantWatchlistRow {
                id: row.get(0)?,
                code: row.get(1)?,
                name: row.get(2)?,
                market: row.get(3)?,
                enabled: enabled != 0,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut quant_strategy_settings_stmt = conn
        .prepare(
            "SELECT id, code, market, ma_short, ma_long, grid_lookback_days, poll_interval_seconds,
                    desktop_notification_enabled, enabled, created_at, updated_at
             FROM quant_strategy_settings ORDER BY id",
        )
        .map_err(|e| e.to_string())?;
    let quant_strategy_settings: Vec<QuantStrategySettingsRow> = quant_strategy_settings_stmt
        .query_map([], |row| {
            let desktop_notification_enabled: i64 = row.get(7)?;
            let enabled: i64 = row.get(8)?;
            Ok(QuantStrategySettingsRow {
                id: row.get(0)?,
                code: row.get(1)?,
                market: row.get(2)?,
                ma_short: row.get(3)?,
                ma_long: row.get(4)?,
                grid_lookback_days: row.get(5)?,
                poll_interval_seconds: row.get(6)?,
                desktop_notification_enabled: desktop_notification_enabled != 0,
                enabled: enabled != 0,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut quant_signals_stmt = conn
        .prepare(
            "SELECT id, code, name, market, direction, trigger_price, trend_state, source,
                    trigger_zone, dedupe_key, triggered_at, created_at
             FROM quant_signals ORDER BY id",
        )
        .map_err(|e| e.to_string())?;
    let quant_signals: Vec<QuantSignalRow> = quant_signals_stmt
        .query_map([], |row| {
            Ok(QuantSignalRow {
                id: row.get(0)?,
                code: row.get(1)?,
                name: row.get(2)?,
                market: row.get(3)?,
                direction: row.get(4)?,
                trigger_price: row.get(5)?,
                trend_state: row.get(6)?,
                source: row.get(7)?,
                trigger_zone: row.get(8)?,
                dedupe_key: row.get(9)?,
                triggered_at: row.get(10)?,
                created_at: row.get(11)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(ExportPayload {
        version: "1.0".to_string(),
        exported_at: now_local(),
        accounts,
        categories,
        transactions,
        holdings,
        settings,
        quant_watchlist,
        quant_strategy_settings,
        quant_signals,
    })
}

#[tauri::command]
pub fn import_data(state: State<AppState>, json: String) -> Result<(), String> {
    let payload: ExportPayload = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    let conn = state.db.lock().map_err(|e| e.to_string())?;

    import_payload_to_conn(&conn, payload)
}

fn import_payload_to_conn(conn: &Connection, payload: ExportPayload) -> Result<(), String> {
    conn.execute_batch(
        "DELETE FROM quant_signals;
         DELETE FROM quant_strategy_settings;
         DELETE FROM quant_watchlist;
         DELETE FROM price_history;
         DELETE FROM transactions;
         DELETE FROM holdings;
         DELETE FROM categories;
         DELETE FROM accounts;
         DELETE FROM settings;",
    )
    .map_err(|e| e.to_string())?;

    for account in payload.accounts {
        conn.execute(
            "INSERT INTO accounts (id, name, type, balance, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![account.id, account.name, account.r#type, account.balance, account.created_at],
        )
        .map_err(|e| e.to_string())?;
    }

    for category in payload.categories {
        conn.execute(
            "INSERT INTO categories (id, name, type, icon) VALUES (?1, ?2, ?3, ?4)",
            params![category.id, category.name, category.r#type, category.icon],
        )
        .map_err(|e| e.to_string())?;
    }

    for tx in payload.transactions {
        conn.execute(
            "INSERT INTO transactions (type, amount, category_id, account_id, transfer_to_account_id, note, transaction_date)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                tx.r#type,
                tx.amount,
                tx.category_id,
                tx.account_id,
                tx.transfer_to_account_id,
                tx.note,
                tx.transaction_date
            ],
        )
        .map_err(|e| e.to_string())?;
    }

    for holding in payload.holdings {
        conn.execute(
            "INSERT INTO holdings (code, name, type, quantity, cost_price, current_price, market)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                holding.code,
                holding.name,
                holding.r#type,
                holding.quantity,
                holding.cost_price,
                holding.current_price,
                holding.market
            ],
        )
        .map_err(|e| e.to_string())?;
    }

    for (key, value) in payload.settings {
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        )
        .map_err(|e| e.to_string())?;
    }

    for item in payload.quant_watchlist {
        conn.execute(
            "INSERT INTO quant_watchlist (id, code, name, market, enabled, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                item.id,
                item.code,
                item.name,
                item.market,
                if item.enabled { 1 } else { 0 },
                item.created_at,
                item.updated_at
            ],
        )
        .map_err(|e| e.to_string())?;
    }

    for settings in payload.quant_strategy_settings {
        conn.execute(
            "INSERT INTO quant_strategy_settings
             (id, code, market, ma_short, ma_long, grid_lookback_days, poll_interval_seconds,
              desktop_notification_enabled, enabled, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                settings.id,
                settings.code,
                settings.market,
                settings.ma_short,
                settings.ma_long,
                settings.grid_lookback_days,
                settings.poll_interval_seconds,
                if settings.desktop_notification_enabled {
                    1
                } else {
                    0
                },
                if settings.enabled { 1 } else { 0 },
                settings.created_at,
                settings.updated_at
            ],
        )
        .map_err(|e| e.to_string())?;
    }

    for signal in payload.quant_signals {
        conn.execute(
            "INSERT INTO quant_signals
             (id, code, name, market, direction, trigger_price, trend_state, source,
              trigger_zone, dedupe_key, triggered_at, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                signal.id,
                signal.code,
                signal.name,
                signal.market,
                signal.direction,
                signal.trigger_price,
                signal.trend_state,
                signal.source,
                signal.trigger_zone,
                signal.dedupe_key,
                signal.triggered_at,
                signal.created_at
            ],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_schema(conn: &Connection) {
        conn.execute_batch(
            "
            CREATE TABLE accounts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                type TEXT NOT NULL DEFAULT 'cash',
                balance REAL NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
            );

            CREATE TABLE categories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                type TEXT NOT NULL,
                icon TEXT DEFAULT 'pushpin'
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
            ",
        )
        .unwrap();
    }

    fn old_backup_json() -> &'static str {
        r#"{
            "version": "1.0",
            "exported_at": "2026-01-01 00:00:00",
            "accounts": [],
            "categories": [],
            "transactions": [],
            "holdings": [],
            "settings": []
        }"#
    }

    fn table_count(conn: &Connection, table: &str) -> i64 {
        conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .unwrap()
    }

    #[test]
    fn export_payload_accepts_missing_quant_fields() {
        let payload: ExportPayload = serde_json::from_str(old_backup_json()).unwrap();

        assert!(payload.quant_watchlist.is_empty());
        assert!(payload.quant_strategy_settings.is_empty());
        assert!(payload.quant_signals.is_empty());
    }

    #[test]
    fn old_backup_import_clears_stale_quant_rows() {
        let conn = Connection::open_in_memory().unwrap();
        setup_schema(&conn);

        conn.execute(
            "INSERT INTO quant_watchlist (id, code, name, market, enabled, created_at, updated_at)
             VALUES (11, '000001', 'Ping An', 'cn', 1, '2026-01-01 09:00:00', '2026-01-01 09:00:00')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO quant_strategy_settings
             (id, code, market, ma_short, ma_long, grid_lookback_days, poll_interval_seconds, desktop_notification_enabled, enabled, created_at, updated_at)
             VALUES (12, '000001', 'cn', 5, 20, 30, 60, 1, 1, '2026-01-01 09:00:00', '2026-01-01 09:00:00')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO quant_signals
             (id, code, name, market, direction, trigger_price, trend_state, source, trigger_zone, dedupe_key, triggered_at, created_at)
             VALUES (13, '000001', 'Ping An', 'cn', 'buy', 10.5, 'up', 'watchlist', 'lower', '000001-cn-buy-2026-01-01', '2026-01-01 09:30:00', '2026-01-01 09:30:01')",
            [],
        )
        .unwrap();

        let payload: ExportPayload = serde_json::from_str(old_backup_json()).unwrap();
        import_payload_to_conn(&conn, payload).unwrap();

        assert_eq!(table_count(&conn, "quant_watchlist"), 0);
        assert_eq!(table_count(&conn, "quant_strategy_settings"), 0);
        assert_eq!(table_count(&conn, "quant_signals"), 0);
    }

    #[test]
    fn export_import_preserves_quant_rows_with_ids() {
        let source = Connection::open_in_memory().unwrap();
        setup_schema(&source);

        source.execute(
            "INSERT INTO quant_watchlist (id, code, name, market, enabled, created_at, updated_at)
             VALUES (21, '513500', 'S&P 500 ETF', 'cn', 0, '2026-02-01 09:00:00', '2026-02-01 09:01:00')",
            [],
        )
        .unwrap();
        source.execute(
            "INSERT INTO quant_strategy_settings
             (id, code, market, ma_short, ma_long, grid_lookback_days, poll_interval_seconds, desktop_notification_enabled, enabled, created_at, updated_at)
             VALUES (22, '513500', 'cn', 8, 34, 55, 120, 0, 1, '2026-02-01 09:02:00', '2026-02-01 09:03:00')",
            [],
        )
        .unwrap();
        source.execute(
            "INSERT INTO quant_signals
             (id, code, name, market, direction, trigger_price, trend_state, source, trigger_zone, dedupe_key, triggered_at, created_at)
             VALUES (23, '513500', 'S&P 500 ETF', 'cn', 'sell', 1.234, 'down', 'scheduler', 'upper', '513500-cn-sell-2026-02-01T10:00:00', '2026-02-01 10:00:00', '2026-02-01 10:00:01')",
            [],
        )
        .unwrap();

        let payload = export_payload_from_conn(&source).unwrap();

        assert_eq!(payload.quant_watchlist[0].id, 21);
        assert_eq!(payload.quant_watchlist[0].code, "513500");
        assert!(!payload.quant_watchlist[0].enabled);
        assert_eq!(payload.quant_strategy_settings[0].id, 22);
        assert_eq!(payload.quant_strategy_settings[0].ma_short, 8);
        assert!(!payload.quant_strategy_settings[0].desktop_notification_enabled);
        assert_eq!(payload.quant_signals[0].id, 23);
        assert_eq!(
            payload.quant_signals[0].dedupe_key,
            "513500-cn-sell-2026-02-01T10:00:00"
        );
        assert_eq!(payload.quant_signals[0].triggered_at, "2026-02-01 10:00:00");

        let target = Connection::open_in_memory().unwrap();
        setup_schema(&target);
        import_payload_to_conn(&target, payload).unwrap();

        let imported_watchlist: (i64, String, String, i64) = target
            .query_row(
                "SELECT id, code, market, enabled FROM quant_watchlist",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert_eq!(
            imported_watchlist,
            (21, "513500".to_string(), "cn".to_string(), 0)
        );

        let imported_settings: (i64, String, i64, i64, i64, i64) = target
            .query_row(
                "SELECT id, code, ma_short, ma_long, desktop_notification_enabled, enabled FROM quant_strategy_settings",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )
            .unwrap();
        assert_eq!(imported_settings, (22, "513500".to_string(), 8, 34, 0, 1));

        let imported_signal: (i64, String, String, String, String) = target
            .query_row(
                "SELECT id, code, direction, dedupe_key, triggered_at FROM quant_signals",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .unwrap();
        assert_eq!(
            imported_signal,
            (
                23,
                "513500".to_string(),
                "sell".to_string(),
                "513500-cn-sell-2026-02-01T10:00:00".to_string(),
                "2026-02-01 10:00:00".to_string(),
            )
        );
    }
}
