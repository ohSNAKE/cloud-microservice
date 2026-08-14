use crate::models::{Category, NewCategory, UpdateCategory};
use crate::AppState;
use rusqlite::params;
use tauri::State;

#[tauri::command]
pub fn add_category(state: State<AppState>, input: NewCategory) -> Result<Category, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let icon = input.icon.unwrap_or_else(|| "pushpin".to_string());
    conn.execute(
        "INSERT INTO categories (name, type, icon) VALUES (?1, ?2, ?3)",
        params![input.name, input.r#type, icon],
    )
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    conn.query_row(
        "SELECT id, name, type, icon FROM categories WHERE id = ?1",
        params![id],
        |row| {
            Ok(Category {
                id: row.get(0)?,
                name: row.get(1)?,
                r#type: row.get(2)?,
                icon: row.get(3)?,
            })
        },
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_category(
    state: State<AppState>,
    id: i64,
    input: UpdateCategory,
) -> Result<Category, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE categories SET name = ?1, icon = ?2 WHERE id = ?3",
        params![input.name, input.icon, id],
    )
    .map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT id, name, type, icon FROM categories WHERE id = ?1",
        params![id],
        |row| {
            Ok(Category {
                id: row.get(0)?,
                name: row.get(1)?,
                r#type: row.get(2)?,
                icon: row.get(3)?,
            })
        },
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_category(state: State<AppState>, id: i64) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let tx_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM transactions WHERE category_id = ?1",
            params![id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if tx_count > 0 {
        return Err("该分类有关联记账记录，无法删除".to_string());
    }
    conn.execute("DELETE FROM categories WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}
