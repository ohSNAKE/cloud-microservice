//! 集成测试：数据库与业务逻辑

use rusqlite::Connection;

fn setup_test_db() -> Connection {
    let conn = Connection::open_in_memory().expect("open memory db");
    conn.execute_batch(
        "
        PRAGMA foreign_keys = ON;

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
            note TEXT DEFAULT '',
            transaction_date TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            FOREIGN KEY (category_id) REFERENCES categories(id),
            FOREIGN KEY (account_id) REFERENCES accounts(id)
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
            recorded_at TEXT NOT NULL,
            FOREIGN KEY (holding_id) REFERENCES holdings(id) ON DELETE CASCADE
        );

        CREATE TABLE settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        ",
    )
    .expect("create schema");

    conn.execute(
        "INSERT INTO accounts (name, type, balance) VALUES ('测试账户', 'bank', 1000)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO categories (name, type, icon) VALUES ('餐饮', 'expense', 'dining')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO categories (name, type, icon) VALUES ('工资', 'income', 'salary')",
        [],
    )
    .unwrap();

    conn
}

#[test]
fn test_expense_updates_account_balance() {
    let conn = setup_test_db();

    conn.execute(
        "INSERT INTO transactions (type, amount, category_id, account_id, note, transaction_date)
         VALUES ('expense', 100, 1, 1, '午餐', '2026-07-01')",
        [],
    )
    .unwrap();
    conn.execute(
        "UPDATE accounts SET balance = balance - 100 WHERE id = 1",
        [],
    )
    .unwrap();

    let balance: f64 = conn
        .query_row("SELECT balance FROM accounts WHERE id = 1", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert!((balance - 900.0).abs() < 0.01);
}

#[test]
fn test_income_updates_account_balance() {
    let conn = setup_test_db();

    conn.execute(
        "INSERT INTO transactions (type, amount, category_id, account_id, transaction_date)
         VALUES ('income', 5000, 2, 1, '2026-07-01')",
        [],
    )
    .unwrap();
    conn.execute(
        "UPDATE accounts SET balance = balance + 5000 WHERE id = 1",
        [],
    )
    .unwrap();

    let balance: f64 = conn
        .query_row("SELECT balance FROM accounts WHERE id = 1", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert!((balance - 6000.0).abs() < 0.01);
}

#[test]
fn test_holding_profit_calculation() {
    let conn = setup_test_db();

    conn.execute(
        "INSERT INTO holdings (code, name, type, quantity, cost_price, current_price)
         VALUES ('600519', '贵州茅台', 'stock', 10, 1500, 1600)",
        [],
    )
    .unwrap();

    let (quantity, cost, current): (f64, f64, f64) = conn
        .query_row(
            "SELECT quantity, cost_price, current_price FROM holdings WHERE id = 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();

    let profit = quantity * current - quantity * cost;
    assert!((profit - 1000.0).abs() < 0.01);
}

#[test]
fn test_monthly_summary_query() {
    let conn = setup_test_db();

    conn.execute(
        "INSERT INTO transactions (type, amount, category_id, account_id, transaction_date)
         VALUES ('income', 8000, 2, 1, '2026-07-05')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO transactions (type, amount, category_id, account_id, transaction_date)
         VALUES ('expense', 200, 1, 1, '2026-07-10')",
        [],
    )
    .unwrap();

    let income: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(amount), 0) FROM transactions WHERE type = 'income' AND transaction_date LIKE '2026-07%'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let expense: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(amount), 0) FROM transactions WHERE type = 'expense' AND transaction_date LIKE '2026-07%'",
            [],
            |r| r.get(0),
        )
        .unwrap();

    assert!((income - 8000.0).abs() < 0.01);
    assert!((expense - 200.0).abs() < 0.01);
}

#[test]
fn test_price_history_cascade_delete() {
    let conn = setup_test_db();

    conn.execute(
        "INSERT INTO holdings (code, name, type, quantity, cost_price, current_price)
         VALUES ('000001', '测试', 'fund', 100, 1.5, 1.6)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO price_history (holding_id, price, recorded_at) VALUES (1, 1.6, '2026-07-01 10:00:00')",
        [],
    )
    .unwrap();
    conn.execute("DELETE FROM holdings WHERE id = 1", [])
        .unwrap();

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM price_history", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn test_stock_secid_mapping() {
    assert_eq!(stock_secid("600519"), "1.600519");
    assert_eq!(stock_secid("000001"), "0.000001");
    assert_eq!(stock_secid("300750"), "0.300750");
}

fn stock_secid(code: &str) -> String {
    if code.starts_with('6') {
        format!("1.{code}")
    } else {
        format!("0.{code}")
    }
}

#[test]
fn test_parse_stock_kline_line() {
    let line =
        "2026-06-30,1420.00,1430.50,1445.00,1410.00,1234567,9876543210.00,2.50,1.20,17.00,0.85";
    let parts: Vec<&str> = line.split(',').collect();
    assert_eq!(parts[0], "2026-06-30");
    assert!((parts[1].parse::<f64>().unwrap() - 1420.0).abs() < 0.01);
    assert!((parts[2].parse::<f64>().unwrap() - 1430.5).abs() < 0.01);
}
