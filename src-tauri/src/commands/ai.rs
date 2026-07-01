use crate::db::get_setting;
use crate::models::{Account, AiConfig, Category, ParsedTransactionDraft};
use crate::services::ai_parser::parse_transaction_nl;
use crate::AppState;
use tauri::State;

fn map_category(row: &rusqlite::Row) -> rusqlite::Result<Category> {
    Ok(Category {
        id: row.get(0)?,
        name: row.get(1)?,
        r#type: row.get(2)?,
        icon: row.get(3)?,
    })
}

fn map_account(row: &rusqlite::Row) -> rusqlite::Result<Account> {
    Ok(Account {
        id: row.get(0)?,
        name: row.get(1)?,
        r#type: row.get(2)?,
        balance: row.get(3)?,
        created_at: row.get(4)?,
    })
}

fn load_ai_config(conn: &rusqlite::Connection) -> AiConfig {
    AiConfig {
        ai_enabled: get_setting(conn, "ai_enabled")
            .map(|v| v == "true")
            .unwrap_or(false),
        ai_api_key: get_setting(conn, "ai_api_key").unwrap_or_default(),
        ai_api_base: get_setting(conn, "ai_api_base")
            .unwrap_or_else(|| "https://v2.pincc.ai/v1".to_string()),
        ai_model: get_setting(conn, "ai_model").unwrap_or_else(|| "gpt-4o-mini".to_string()),
    }
}

#[tauri::command]
pub fn parse_transaction_nl_command(
    state: State<AppState>,
    text: String,
) -> Result<ParsedTransactionDraft, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;

    let categories = {
        let mut stmt = conn
            .prepare("SELECT id, name, type, icon FROM categories ORDER BY id")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], map_category)
            .map_err(|e| e.to_string())?;
        rows.filter_map(Result::ok).collect::<Vec<_>>()
    };

    let accounts = {
        let mut stmt = conn
            .prepare("SELECT id, name, type, balance, created_at FROM accounts ORDER BY id")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], map_account)
            .map_err(|e| e.to_string())?;
        rows.filter_map(Result::ok).collect::<Vec<_>>()
    };

    let config = load_ai_config(&conn);

    parse_transaction_nl(text.trim(), &categories, &accounts, &config)
}
