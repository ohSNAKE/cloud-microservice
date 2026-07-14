use crate::db::{db_path_hint, now_local};
use crate::models::{
    Account, Category, ExportPayload, HoldingRow, QuantIntradayTStateRow, QuantSignalRow,
    QuantStrategySettingsRow, QuantWatchlistRow, TransactionRow,
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
                    desktop_notification_enabled, enabled, strategy_mode, intraday_lookback_days,
                    intraday_high_time_min_count, intraday_low_time_min_count,
                    sell_t_position_threshold, buyback_position_threshold, created_at, updated_at
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
                strategy_mode: row.get(9)?,
                intraday_lookback_days: row.get(10)?,
                intraday_high_time_min_count: row.get(11)?,
                intraday_low_time_min_count: row.get(12)?,
                sell_t_position_threshold: row.get(13)?,
                buyback_position_threshold: row.get(14)?,
                created_at: row.get(15)?,
                updated_at: row.get(16)?,
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

    let mut quant_intraday_t_state_stmt = conn
        .prepare(
            "SELECT code, market, trading_date, sold_at
             FROM quant_intraday_t_state ORDER BY code, market, trading_date",
        )
        .map_err(|e| e.to_string())?;
    let quant_intraday_t_state: Vec<QuantIntradayTStateRow> = quant_intraday_t_state_stmt
        .query_map([], |row| {
            Ok(QuantIntradayTStateRow {
                code: row.get(0)?,
                market: row.get(1)?,
                trading_date: row.get(2)?,
                sold_at: row.get(3)?,
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
        quant_intraday_t_state,
    })
}

#[tauri::command]
pub fn import_data(state: State<AppState>, json: String) -> Result<(), String> {
    let payload: ExportPayload = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    let conn = state.db.lock().map_err(|e| e.to_string())?;

    import_payload_to_conn(&conn, payload)
}

fn import_payload_to_conn(conn: &Connection, payload: ExportPayload) -> Result<(), String> {
    for settings in &payload.quant_strategy_settings {
        match settings.strategy_mode.as_str() {
            "auto_grid" | "intraday_t" => {}
            _ => return Err("invalid strategy_mode; expected auto_grid or intraday_t".to_string()),
        }
    }

    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    tx.execute_batch(
        "DELETE FROM quant_intraday_t_state;
         DELETE FROM quant_signals;
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
        tx.execute(
            "INSERT INTO accounts (id, name, type, balance, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![account.id, account.name, account.r#type, account.balance, account.created_at],
        )
        .map_err(|e| e.to_string())?;
    }

    for category in payload.categories {
        tx.execute(
            "INSERT INTO categories (id, name, type, icon) VALUES (?1, ?2, ?3, ?4)",
            params![category.id, category.name, category.r#type, category.icon],
        )
        .map_err(|e| e.to_string())?;
    }

    for transaction in payload.transactions {
        tx.execute(
            "INSERT INTO transactions (type, amount, category_id, account_id, transfer_to_account_id, note, transaction_date)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                transaction.r#type,
                transaction.amount,
                transaction.category_id,
                transaction.account_id,
                transaction.transfer_to_account_id,
                transaction.note,
                transaction.transaction_date
            ],
        )
        .map_err(|e| e.to_string())?;
    }

    for holding in payload.holdings {
        tx.execute(
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
        tx.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        )
        .map_err(|e| e.to_string())?;
    }

    for item in payload.quant_watchlist {
        tx.execute(
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
        tx.execute(
            "INSERT INTO quant_strategy_settings
             (id, code, market, ma_short, ma_long, grid_lookback_days, poll_interval_seconds,
              desktop_notification_enabled, enabled, strategy_mode, intraday_lookback_days,
              intraday_high_time_min_count, intraday_low_time_min_count,
              sell_t_position_threshold, buyback_position_threshold, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
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
                settings.strategy_mode,
                settings.intraday_lookback_days,
                settings.intraday_high_time_min_count,
                settings.intraday_low_time_min_count,
                settings.sell_t_position_threshold,
                settings.buyback_position_threshold,
                settings.created_at,
                settings.updated_at
            ],
        )
        .map_err(|e| e.to_string())?;
    }

    for signal in payload.quant_signals {
        tx.execute(
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

    for state in payload.quant_intraday_t_state {
        tx.execute(
            "INSERT INTO quant_intraday_t_state (code, market, trading_date, sold_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![state.code, state.market, state.trading_date, state.sold_at],
        )
        .map_err(|e| e.to_string())?;
    }

    tx.commit().map_err(|e| e.to_string())
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

    fn old_backup_json_with_legacy_quant_settings() -> &'static str {
        r#"{
            "version": "1.0",
            "exported_at": "2026-01-01 00:00:00",
            "accounts": [],
            "categories": [],
            "transactions": [],
            "holdings": [],
            "settings": [],
            "quant_watchlist": [],
            "quant_strategy_settings": [
                {
                    "id": 12,
                    "code": "000001",
                    "market": "cn",
                    "ma_short": 5,
                    "ma_long": 20,
                    "grid_lookback_days": 30,
                    "poll_interval_seconds": 60,
                    "desktop_notification_enabled": true,
                    "enabled": true,
                    "created_at": "2026-01-01 09:00:00",
                    "updated_at": "2026-01-01 09:00:00"
                }
            ],
            "quant_signals": []
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
        assert!(payload.quant_intraday_t_state.is_empty());
    }

    #[test]
    fn legacy_quant_settings_backup_uses_intraday_defaults() {
        let payload: ExportPayload =
            serde_json::from_str(old_backup_json_with_legacy_quant_settings()).unwrap();

        let settings = &payload.quant_strategy_settings[0];
        assert_eq!(settings.strategy_mode, "auto_grid");
        assert_eq!(settings.intraday_lookback_days, 22);
        assert_eq!(settings.intraday_high_time_min_count, 4);
        assert_eq!(settings.intraday_low_time_min_count, 4);
        assert_eq!(settings.sell_t_position_threshold, 0.70);
        assert_eq!(settings.buyback_position_threshold, 0.30);
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
             (id, code, market, ma_short, ma_long, grid_lookback_days, poll_interval_seconds,
              desktop_notification_enabled, enabled, strategy_mode, intraday_lookback_days,
              intraday_high_time_min_count, intraday_low_time_min_count,
              sell_t_position_threshold, buyback_position_threshold, created_at, updated_at)
             VALUES (12, '000001', 'cn', 5, 20, 30, 60, 1, 1, 'auto_grid', 22, 4, 4, 0.70, 0.30, '2026-01-01 09:00:00', '2026-01-01 09:00:00')",
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
        conn.execute(
            "INSERT INTO quant_intraday_t_state (code, market, trading_date, sold_at)
             VALUES ('000001', 'cn', '2026-01-01', '2026-01-01 09:30:00')",
            [],
        )
        .unwrap();

        let payload: ExportPayload = serde_json::from_str(old_backup_json()).unwrap();
        import_payload_to_conn(&conn, payload).unwrap();

        assert_eq!(table_count(&conn, "quant_watchlist"), 0);
        assert_eq!(table_count(&conn, "quant_strategy_settings"), 0);
        assert_eq!(table_count(&conn, "quant_signals"), 0);
        assert_eq!(table_count(&conn, "quant_intraday_t_state"), 0);
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
             (id, code, market, ma_short, ma_long, grid_lookback_days, poll_interval_seconds,
              desktop_notification_enabled, enabled, strategy_mode, intraday_lookback_days,
              intraday_high_time_min_count, intraday_low_time_min_count,
              sell_t_position_threshold, buyback_position_threshold, created_at, updated_at)
             VALUES (22, '513500', 'cn', 8, 34, 55, 120, 0, 1, 'intraday_t', 33, 6, 5, 0.82, 0.18, '2026-02-01 09:02:00', '2026-02-01 09:03:00')",
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
        assert_eq!(
            payload.quant_strategy_settings[0].strategy_mode,
            "intraday_t"
        );
        assert_eq!(
            payload.quant_strategy_settings[0].intraday_lookback_days,
            33
        );
        assert_eq!(
            payload.quant_strategy_settings[0].intraday_high_time_min_count,
            6
        );
        assert_eq!(
            payload.quant_strategy_settings[0].intraday_low_time_min_count,
            5
        );
        assert_eq!(
            payload.quant_strategy_settings[0].sell_t_position_threshold,
            0.82
        );
        assert_eq!(
            payload.quant_strategy_settings[0].buyback_position_threshold,
            0.18
        );
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

        let imported_settings: (
            i64,
            String,
            i64,
            i64,
            i64,
            i64,
            String,
            i64,
            i64,
            i64,
            f64,
            f64,
        ) = target
            .query_row(
                "SELECT id, code, ma_short, ma_long, desktop_notification_enabled, enabled,
                        strategy_mode, intraday_lookback_days, intraday_high_time_min_count,
                        intraday_low_time_min_count, sell_t_position_threshold,
                        buyback_position_threshold
                 FROM quant_strategy_settings",
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
                        row.get(8)?,
                        row.get(9)?,
                        row.get(10)?,
                        row.get(11)?,
                    ))
                },
            )
            .unwrap();
        assert_eq!(
            imported_settings,
            (
                22,
                "513500".to_string(),
                8,
                34,
                0,
                1,
                "intraday_t".to_string(),
                33,
                6,
                5,
                0.82,
                0.18,
            )
        );

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

    #[test]
    fn export_import_round_trips_intraday_t_sale_acknowledgement() {
        let source = Connection::open_in_memory().unwrap();
        setup_schema(&source);
        source
            .execute(
                "INSERT INTO quant_intraday_t_state (code, market, trading_date, sold_at)
                 VALUES ('000001', 'cn', '2026-07-07', '2026-07-07 10:01:02')",
                [],
            )
            .unwrap();

        let payload = export_payload_from_conn(&source).unwrap();
        assert_eq!(payload.quant_intraday_t_state.len(), 1);

        let target = Connection::open_in_memory().unwrap();
        setup_schema(&target);
        import_payload_to_conn(&target, payload).unwrap();
        let imported: (String, String, String, String) = target
            .query_row(
                "SELECT code, market, trading_date, sold_at FROM quant_intraday_t_state",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert_eq!(
            imported,
            (
                "000001".to_string(),
                "cn".to_string(),
                "2026-07-07".to_string(),
                "2026-07-07 10:01:02".to_string(),
            )
        );
    }

    #[test]
    fn failed_late_intraday_t_state_import_leaves_existing_data_unchanged() {
        let source = Connection::open_in_memory().unwrap();
        setup_schema(&source);
        source
            .execute(
                "INSERT INTO accounts (id, name, type, balance, created_at)
                 VALUES (1, 'Imported cash', 'cash', 10.0, '2026-07-07 09:00:00')",
                [],
            )
            .unwrap();
        source
            .execute(
                "INSERT INTO quant_intraday_t_state (code, market, trading_date, sold_at)
                 VALUES ('000001', 'cn', '2026-07-07', '2026-07-07 10:01:02')",
                [],
            )
            .unwrap();
        let mut payload = export_payload_from_conn(&source).unwrap();
        payload
            .quant_intraday_t_state
            .push(payload.quant_intraday_t_state[0].clone());

        let target = Connection::open_in_memory().unwrap();
        setup_schema(&target);
        target
            .execute(
                "INSERT INTO accounts (id, name, type, balance, created_at)
                 VALUES (9, 'Existing cash', 'cash', 99.0, '2026-07-07 08:00:00')",
                [],
            )
            .unwrap();
        target
            .execute(
                "INSERT INTO quant_intraday_t_state (code, market, trading_date, sold_at)
                 VALUES ('000009', 'cn', '2026-07-07', '2026-07-07 08:30:00')",
                [],
            )
            .unwrap();

        assert!(import_payload_to_conn(&target, payload).is_err());

        let account: (i64, String, f64) = target
            .query_row("SELECT id, name, balance FROM accounts", [], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .unwrap();
        assert_eq!(account, (9, "Existing cash".to_string(), 99.0));
        let state: (String, String) = target
            .query_row(
                "SELECT code, sold_at FROM quant_intraday_t_state",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(
            state,
            ("000009".to_string(), "2026-07-07 08:30:00".to_string())
        );
    }

    #[test]
    fn invalid_strategy_mode_import_returns_error_without_replacing_existing_data() {
        let source = Connection::open_in_memory().unwrap();
        setup_schema(&source);
        source
            .execute(
                "INSERT INTO quant_strategy_settings
                 (id, code, market, strategy_mode, created_at, updated_at)
                 VALUES (1, '000001', 'cn', 'unsupported_mode', '2026-07-07 09:00:00', '2026-07-07 09:00:00')",
                [],
            )
            .unwrap();
        let payload = export_payload_from_conn(&source).unwrap();

        let target = Connection::open_in_memory().unwrap();
        setup_schema(&target);
        target
            .execute(
                "INSERT INTO accounts (id, name, type, balance, created_at)
                 VALUES (9, 'Existing cash', 'cash', 99.0, '2026-07-07 08:00:00')",
                [],
            )
            .unwrap();

        let error = import_payload_to_conn(&target, payload).unwrap_err();

        assert_eq!(
            error,
            "invalid strategy_mode; expected auto_grid or intraday_t"
        );
        let account: (i64, String, f64) = target
            .query_row("SELECT id, name, balance FROM accounts", [], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .unwrap();
        assert_eq!(account, (9, "Existing cash".to_string(), 99.0));
    }
}
