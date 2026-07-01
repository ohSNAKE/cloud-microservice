use crate::db::{db_path_hint, now_local};
use crate::models::{
    Account, Category, ExportPayload, HoldingRow, TransactionRow,
};
use crate::AppState;
use rusqlite::params;
use tauri::State;

#[tauri::command]
pub fn get_db_path(app: tauri::AppHandle) -> Result<String, String> {
    Ok(db_path_hint(&app).to_string_lossy().to_string())
}

#[tauri::command]
pub fn export_data(state: State<AppState>) -> Result<String, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;

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

    let payload = ExportPayload {
        version: "1.0".to_string(),
        exported_at: now_local(),
        accounts,
        categories,
        transactions,
        holdings,
        settings,
    };

    serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn import_data(state: State<AppState>, json: String) -> Result<(), String> {
    let payload: ExportPayload = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    let conn = state.db.lock().map_err(|e| e.to_string())?;

    conn.execute_batch(
        "DELETE FROM price_history;
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

    Ok(())
}
