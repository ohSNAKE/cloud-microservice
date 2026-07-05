use crate::models::{NewAccount, UpdateAccount, Account};
use crate::AppState;
use rusqlite::params;
use tauri::State;

#[tauri::command]
pub fn add_account(state: State<AppState>, input: NewAccount) -> Result<Account, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let balance = input.balance.unwrap_or(0.0);
    conn.execute(
        "INSERT INTO accounts (name, type, balance) VALUES (?1, ?2, ?3)",
        params![input.name, input.r#type, balance],
    )
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    conn.query_row(
        "SELECT id, name, type, balance, created_at FROM accounts WHERE id = ?1",
        params![id],
        |row| {
            Ok(Account {
                id: row.get(0)?,
                name: row.get(1)?,
                r#type: row.get(2)?,
                balance: row.get(3)?,
                created_at: row.get(4)?,
            })
        },
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_account(state: State<AppState>, id: i64, input: UpdateAccount) -> Result<Account, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE accounts SET name = ?1, type = ?2, balance = ?3 WHERE id = ?4",
        params![input.name, input.r#type, input.balance, id],
    )
    .map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT id, name, type, balance, created_at FROM accounts WHERE id = ?1",
        params![id],
        |row| {
            Ok(Account {
                id: row.get(0)?,
                name: row.get(1)?,
                r#type: row.get(2)?,
                balance: row.get(3)?,
                created_at: row.get(4)?,
            })
        },
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_account(state: State<AppState>, id: i64) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let tx_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM transactions WHERE account_id = ?1 OR transfer_to_account_id = ?1",
            params![id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if tx_count > 0 {
        return Err("该账户有关联记账记录，无法删除。可先删除相关流水，或保留账户。".to_string());
    }
    let recurring_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM recurring_rules WHERE account_id = ?1",
            params![id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if recurring_count > 0 {
        return Err(
            "该账户已绑定周期记账（如工资），请先在「设置 → 月薪设置」或「记账 → 周期记账」中解除后再删除。"
                .to_string(),
        );
    }
    let affected = conn
        .execute("DELETE FROM accounts WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    if affected == 0 {
        return Err("账户不存在或已被删除".to_string());
    }
    Ok(())
}
