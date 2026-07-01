use crate::db::current_month;
use crate::models::CategoryStat;
use crate::AppState;
use rusqlite::params;
use tauri::State;

#[tauri::command]
pub fn get_category_stats(
    state: State<AppState>,
    month: Option<String>,
    kind: Option<String>,
) -> Result<Vec<CategoryStat>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let filter_month = month.unwrap_or_else(current_month);
    let pattern = format!("{filter_month}%");
    let tx_type = kind.unwrap_or_else(|| "expense".to_string());

    let total: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(amount), 0) FROM transactions WHERE type = ?1 AND transaction_date LIKE ?2",
            params![tx_type, pattern],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT c.id, c.name, c.icon, COALESCE(SUM(t.amount), 0) as amount
             FROM categories c
             LEFT JOIN transactions t ON t.category_id = c.id AND t.type = ?1 AND t.transaction_date LIKE ?2
             WHERE c.type = ?1
             GROUP BY c.id
             HAVING amount > 0
             ORDER BY amount DESC",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(params![tx_type, pattern], |row| {
            let amount: f64 = row.get(3)?;
            let percentage = if total > 0.0 {
                (amount / total) * 100.0
            } else {
                0.0
            };
            Ok(CategoryStat {
                category_id: row.get(0)?,
                category_name: row.get(1)?,
                category_icon: row.get(2)?,
                amount,
                percentage,
            })
        })
        .map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}
