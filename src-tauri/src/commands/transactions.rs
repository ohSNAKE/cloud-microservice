use crate::db::{current_month, now_local};
use crate::models::{
    Account, Category, DashboardSummary, MonthlyStat, NewTransaction, Transaction,
};
use crate::AppState;
use rusqlite::params;
use tauri::State;

#[tauri::command]
pub fn list_accounts(state: State<AppState>) -> Result<Vec<Account>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, name, type, balance, created_at FROM accounts ORDER BY id")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(Account {
                id: row.get(0)?,
                name: row.get(1)?,
                r#type: row.get(2)?,
                balance: row.get(3)?,
                created_at: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_categories(state: State<AppState>, kind: Option<String>) -> Result<Vec<Category>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let mut sql = String::from("SELECT id, name, type, icon FROM categories");
    if kind.is_some() {
        sql.push_str(" WHERE type = ?1");
    }
    sql.push_str(" ORDER BY id");

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = if let Some(ref t) = kind {
        stmt.query_map(params![t], map_category).map_err(|e| e.to_string())?
    } else {
        stmt.query_map([], map_category).map_err(|e| e.to_string())?
    };
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

fn map_category(row: &rusqlite::Row) -> rusqlite::Result<Category> {
    Ok(Category {
        id: row.get(0)?,
        name: row.get(1)?,
        r#type: row.get(2)?,
        icon: row.get(3)?,
    })
}

#[tauri::command]
pub fn list_transactions(
    state: State<AppState>,
    month: Option<String>,
) -> Result<Vec<Transaction>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let filter_month = month.unwrap_or_else(current_month);
    let pattern = format!("{filter_month}%");

    let mut stmt = conn
        .prepare(
            "SELECT t.id, t.type, t.amount, t.category_id, t.account_id, t.note,
                    t.transaction_date, t.created_at,
                    c.name, c.icon, a.name
             FROM transactions t
             LEFT JOIN categories c ON t.category_id = c.id
             LEFT JOIN accounts a ON t.account_id = a.id
             WHERE t.transaction_date LIKE ?1
             ORDER BY t.transaction_date DESC, t.id DESC",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(params![pattern], |row| {
            Ok(Transaction {
                id: row.get(0)?,
                r#type: row.get(1)?,
                amount: row.get(2)?,
                category_id: row.get(3)?,
                account_id: row.get(4)?,
                note: row.get(5)?,
                transaction_date: row.get(6)?,
                created_at: row.get(7)?,
                category_name: row.get(8)?,
                category_icon: row.get(9)?,
                account_name: row.get(10)?,
            })
        })
        .map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_transaction(state: State<AppState>, input: NewTransaction) -> Result<Transaction, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let note = input.note.unwrap_or_default();

    conn.execute(
        "INSERT INTO transactions (type, amount, category_id, account_id, note, transaction_date)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            input.r#type,
            input.amount,
            input.category_id,
            input.account_id,
            note,
            input.transaction_date
        ],
    )
    .map_err(|e| e.to_string())?;

    if let Some(account_id) = input.account_id {
        let delta = if input.r#type == "income" {
            input.amount
        } else {
            -input.amount
        };
        conn.execute(
            "UPDATE accounts SET balance = balance + ?1 WHERE id = ?2",
            params![delta, account_id],
        )
        .map_err(|e| e.to_string())?;
    }

    let id = conn.last_insert_rowid();
    drop(conn);
    list_transactions(state, Some(input.transaction_date[..7].to_string()))?
        .into_iter()
        .find(|t| t.id == id)
        .ok_or_else(|| "创建记录失败".to_string())
}

#[tauri::command]
pub fn delete_transaction(state: State<AppState>, id: i64) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let (tx_type, amount, account_id): (String, f64, Option<i64>) = conn
        .query_row(
            "SELECT type, amount, account_id FROM transactions WHERE id = ?1",
            params![id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|e| e.to_string())?;

    if let Some(account_id) = account_id {
        let delta = if tx_type == "income" {
            -amount
        } else {
            amount
        };
        conn.execute(
            "UPDATE accounts SET balance = balance + ?1 WHERE id = ?2",
            params![delta, account_id],
        )
        .map_err(|e| e.to_string())?;
    }

    conn.execute("DELETE FROM transactions WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_dashboard(state: State<AppState>) -> Result<DashboardSummary, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let month = current_month();
    let pattern = format!("{month}%");

    let account_balance: f64 = conn
        .query_row("SELECT COALESCE(SUM(balance), 0) FROM accounts", [], |row| {
            row.get(0)
        })
        .map_err(|e| e.to_string())?;

    let holding_value: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(quantity * current_price), 0) FROM holdings",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    let holding_cost: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(quantity * cost_price), 0) FROM holdings",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    let month_income: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(amount), 0) FROM transactions WHERE type = 'income' AND transaction_date LIKE ?1",
            params![pattern],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    let month_expense: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(amount), 0) FROM transactions WHERE type = 'expense' AND transaction_date LIKE ?1",
            params![pattern],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    let holding_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM holdings", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;

    let transaction_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM transactions WHERE transaction_date LIKE ?1",
            params![pattern],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    Ok(DashboardSummary {
        total_assets: account_balance + holding_value,
        account_balance,
        holding_value,
        holding_profit: holding_value - holding_cost,
        month_income,
        month_expense,
        month_balance: month_income - month_expense,
        holding_count,
        transaction_count,
    })
}

#[tauri::command]
pub fn get_monthly_stats(state: State<AppState>, months: Option<i64>) -> Result<Vec<MonthlyStat>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let limit = months.unwrap_or(6);

    let mut stmt = conn
        .prepare(
            "SELECT strftime('%Y-%m', transaction_date) as month,
                    SUM(CASE WHEN type = 'income' THEN amount ELSE 0 END) as income,
                    SUM(CASE WHEN type = 'expense' THEN amount ELSE 0 END) as expense
             FROM transactions
             GROUP BY month
             ORDER BY month DESC
             LIMIT ?1",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(params![limit], |row| {
            Ok(MonthlyStat {
                month: row.get(0)?,
                income: row.get(1)?,
                expense: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut stats: Vec<MonthlyStat> = rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
    stats.reverse();
    Ok(stats)
}

#[allow(dead_code)]
pub fn touch_updated_at(conn: &rusqlite::Connection) -> Result<(), String> {
    conn.execute(
        "UPDATE settings SET value = ?1 WHERE key = 'last_sync_at'",
        params![now_local()],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
