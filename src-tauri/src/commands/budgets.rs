use crate::db::current_month;
use crate::models::{Budget, BudgetAlert, NewBudget};
use crate::AppState;
use rusqlite::params;
use tauri::State;

fn enrich_budget(row: &rusqlite::Row, month: &str) -> rusqlite::Result<Budget> {
    let category_id: i64 = row.get(1)?;
    let amount: f64 = row.get(3)?;
    let spent: f64 = row.get(6)?;
    let remaining = amount - spent;
    let usage_rate = if amount > 0.0 { (spent / amount) * 100.0 } else { 0.0 };
    Ok(Budget {
        id: row.get(0)?,
        category_id,
        category_name: row.get(4)?,
        category_icon: row.get(5)?,
        month: month.to_string(),
        amount,
        spent,
        remaining,
        usage_rate,
        is_over: spent > amount,
    })
}

#[tauri::command]
pub fn list_budgets(state: State<AppState>, month: Option<String>) -> Result<Vec<Budget>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let filter_month = month.unwrap_or_else(current_month);
    let pattern = format!("{filter_month}%");

    let mut stmt = conn
        .prepare(
            "SELECT b.id, b.category_id, b.month, b.amount,
                    c.name, c.icon,
                    COALESCE((
                        SELECT SUM(t.amount) FROM transactions t
                        WHERE t.category_id = b.category_id AND t.type = 'expense'
                        AND t.transaction_date LIKE ?1
                    ), 0) as spent
             FROM budgets b
             JOIN categories c ON b.category_id = c.id
             WHERE b.month = ?2
             ORDER BY spent DESC",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(params![pattern, filter_month], |row| enrich_budget(row, &filter_month))
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_budget(state: State<AppState>, input: NewBudget) -> Result<Budget, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO budgets (category_id, month, amount) VALUES (?1, ?2, ?3)
         ON CONFLICT(category_id, month) DO UPDATE SET amount = excluded.amount",
        params![input.category_id, input.month, input.amount],
    )
    .map_err(|e| e.to_string())?;
    drop(conn);
    list_budgets(state, Some(input.month))?
        .into_iter()
        .find(|b| b.category_id == input.category_id)
        .ok_or_else(|| "设置预算失败".to_string())
}

#[tauri::command]
pub fn delete_budget(state: State<AppState>, id: i64) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM budgets WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_budget_alerts(state: State<AppState>, month: Option<String>) -> Result<Vec<BudgetAlert>, String> {
    let budgets = list_budgets(state, month)?;
    Ok(budgets
        .into_iter()
        .filter(|b| b.is_over)
        .map(|b| BudgetAlert {
            category_name: b.category_name,
            category_icon: b.category_icon,
            budget: b.amount,
            spent: b.spent,
            over_amount: b.spent - b.amount,
        })
        .collect())
}
