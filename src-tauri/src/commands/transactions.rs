use crate::db::current_month;
use crate::models::{
    Account, Category, DashboardSummary, MonthlyStat, NewTransaction, Transaction,
    TransactionFilter, TransferInput, UpdateTransaction,
};
use crate::AppState;
use rusqlite::params;
use tauri::State;

fn map_category(row: &rusqlite::Row) -> rusqlite::Result<Category> {
    Ok(Category {
        id: row.get(0)?,
        name: row.get(1)?,
        r#type: row.get(2)?,
        icon: row.get(3)?,
    })
}

fn map_transaction(row: &rusqlite::Row) -> rusqlite::Result<Transaction> {
    Ok(Transaction {
        id: row.get(0)?,
        r#type: row.get(1)?,
        amount: row.get(2)?,
        category_id: row.get(3)?,
        account_id: row.get(4)?,
        transfer_to_account_id: row.get(5)?,
        note: row.get(6)?,
        transaction_date: row.get(7)?,
        created_at: row.get(8)?,
        category_name: row.get(9)?,
        category_icon: row.get(10)?,
        account_name: row.get(11)?,
        transfer_to_account_name: row.get(12)?,
    })
}

const TX_SELECT: &str = "
    SELECT t.id, t.type, t.amount, t.category_id, t.account_id, t.transfer_to_account_id,
           t.note, t.transaction_date, t.created_at,
           c.name, c.icon, a.name, a2.name
    FROM transactions t
    LEFT JOIN categories c ON t.category_id = c.id
    LEFT JOIN accounts a ON t.account_id = a.id
    LEFT JOIN accounts a2 ON t.transfer_to_account_id = a2.id
";

fn apply_balance_delta(
    conn: &rusqlite::Connection,
    tx_type: &str,
    amount: f64,
    account_id: Option<i64>,
    transfer_to: Option<i64>,
    reverse: bool,
) -> Result<(), String> {
    let sign = if reverse { -1.0 } else { 1.0 };
    match tx_type {
        "income" => {
            if let Some(id) = account_id {
                conn.execute(
                    "UPDATE accounts SET balance = balance + ?1 WHERE id = ?2",
                    params![sign * amount, id],
                )
                .map_err(|e| e.to_string())?;
            }
        }
        "expense" => {
            if let Some(id) = account_id {
                conn.execute(
                    "UPDATE accounts SET balance = balance - ?1 WHERE id = ?2",
                    params![sign * amount, id],
                )
                .map_err(|e| e.to_string())?;
            }
        }
        "transfer" => {
            if let (Some(from), Some(to)) = (account_id, transfer_to) {
                conn.execute(
                    "UPDATE accounts SET balance = balance - ?1 WHERE id = ?2",
                    params![sign * amount, from],
                )
                .map_err(|e| e.to_string())?;
                conn.execute(
                    "UPDATE accounts SET balance = balance + ?1 WHERE id = ?2",
                    params![sign * amount, to],
                )
                .map_err(|e| e.to_string())?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn fetch_transaction_by_id(conn: &rusqlite::Connection, id: i64) -> Result<Transaction, String> {
    conn.query_row(
        &format!("{TX_SELECT} WHERE t.id = ?1"),
        params![id],
        map_transaction,
    )
    .map_err(|e| e.to_string())
}

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

#[tauri::command]
pub fn list_transactions(
    state: State<AppState>,
    filter: Option<TransactionFilter>,
) -> Result<Vec<Transaction>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let f = filter.unwrap_or_default();
    let month = f.month.unwrap_or_else(current_month);
    let pattern = format!("{month}%");

    let mut sql = format!("{TX_SELECT} WHERE t.transaction_date LIKE ?1");
    let mut bind: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(pattern)];

    if let Some(account_id) = f.account_id {
        sql.push_str(&format!(" AND t.account_id = ?{}", bind.len() + 1));
        bind.push(Box::new(account_id));
    }
    if let Some(category_id) = f.category_id {
        sql.push_str(&format!(" AND t.category_id = ?{}", bind.len() + 1));
        bind.push(Box::new(category_id));
    }
    if let Some(ref tx_type) = f.tx_type {
        sql.push_str(&format!(" AND t.type = ?{}", bind.len() + 1));
        bind.push(Box::new(tx_type.clone()));
    }
    if let Some(keyword) = f.keyword.filter(|k| !k.is_empty()) {
        let kw = format!("%{keyword}%");
        sql.push_str(&format!(
            " AND (t.note LIKE ?{} OR c.name LIKE ?{} OR a.name LIKE ?{})",
            bind.len() + 1,
            bind.len() + 1,
            bind.len() + 1
        ));
        bind.push(Box::new(kw));
    }
    sql.push_str(" ORDER BY t.transaction_date DESC, t.id DESC");

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let params: Vec<&dyn rusqlite::types::ToSql> = bind.iter().map(|b| b.as_ref()).collect();
    let rows = stmt
        .query_map(params.as_slice(), map_transaction)
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

    apply_balance_delta(
        &conn,
        &input.r#type,
        input.amount,
        input.account_id,
        None,
        false,
    )?;

    let id = conn.last_insert_rowid();
    fetch_transaction_by_id(&conn, id)
}

#[tauri::command]
pub fn update_transaction(
    state: State<AppState>,
    id: i64,
    input: UpdateTransaction,
) -> Result<Transaction, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let old = fetch_transaction_by_id(&conn, id)?;

    apply_balance_delta(
        &conn,
        &old.r#type,
        old.amount,
        old.account_id,
        old.transfer_to_account_id,
        true,
    )?;

    let note = input.note.unwrap_or_default();
    conn.execute(
        "UPDATE transactions SET type = ?1, amount = ?2, category_id = ?3,
         account_id = ?4, note = ?5, transaction_date = ?6 WHERE id = ?7",
        params![
            input.r#type,
            input.amount,
            input.category_id,
            input.account_id,
            note,
            input.transaction_date,
            id
        ],
    )
    .map_err(|e| e.to_string())?;

    apply_balance_delta(
        &conn,
        &input.r#type,
        input.amount,
        input.account_id,
        None,
        false,
    )?;

    fetch_transaction_by_id(&conn, id)
}

#[tauri::command]
pub fn add_transfer(state: State<AppState>, input: TransferInput) -> Result<Transaction, String> {
    if input.from_account_id == input.to_account_id {
        return Err("转出和转入账户不能相同".to_string());
    }
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let note = input.note.unwrap_or_default();

    conn.execute(
        "INSERT INTO transactions (type, amount, account_id, transfer_to_account_id, note, transaction_date)
         VALUES ('transfer', ?1, ?2, ?3, ?4, ?5)",
        params![
            input.amount,
            input.from_account_id,
            input.to_account_id,
            note,
            input.transaction_date
        ],
    )
    .map_err(|e| e.to_string())?;

    apply_balance_delta(
        &conn,
        "transfer",
        input.amount,
        Some(input.from_account_id),
        Some(input.to_account_id),
        false,
    )?;

    let id = conn.last_insert_rowid();
    fetch_transaction_by_id(&conn, id)
}

#[tauri::command]
pub fn delete_transaction(state: State<AppState>, id: i64) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let old = fetch_transaction_by_id(&conn, id)?;

    apply_balance_delta(
        &conn,
        &old.r#type,
        old.amount,
        old.account_id,
        old.transfer_to_account_id,
        true,
    )?;

    conn.execute("DELETE FROM transactions WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_dashboard(state: State<AppState>) -> Result<DashboardSummary, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let month = current_month();
    let pattern = format!("{month}%");

    let liquid_assets: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(balance), 0) FROM accounts WHERE type != 'broker'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    let broker_balance: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(balance), 0) FROM accounts WHERE type = 'broker'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

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

    let total_tx: i64 = conn
        .query_row("SELECT COUNT(*) FROM transactions", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;

    Ok(DashboardSummary {
        total_assets: liquid_assets + holding_value,
        liquid_assets,
        broker_balance,
        account_balance,
        holding_value,
        holding_profit: holding_value - holding_cost,
        month_income,
        month_expense,
        month_balance: month_income - month_expense,
        holding_count,
        transaction_count,
        is_empty: total_tx == 0 && holding_count == 0,
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
             WHERE type IN ('income', 'expense')
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
