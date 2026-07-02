use crate::db::{current_month, now_local, today};
use crate::models::{NewRecurringRule, RecurringRule, UpdateRecurringRule};
use crate::AppState;
use rusqlite::params;
use tauri::State;

fn map_rule(row: &rusqlite::Row) -> rusqlite::Result<RecurringRule> {
    let enabled: i64 = row.get(7)?;
    Ok(RecurringRule {
        id: row.get(0)?,
        r#type: row.get(1)?,
        amount: row.get(2)?,
        category_id: row.get(3)?,
        account_id: row.get(4)?,
        note: row.get(5)?,
        day_of_month: row.get(6)?,
        enabled: enabled == 1,
        last_run_month: row.get(8)?,
        category_name: row.get(9)?,
        category_icon: row.get(10)?,
        account_name: row.get(11)?,
    })
}

const RULE_SELECT: &str = "
    SELECT r.id, r.type, r.amount, r.category_id, r.account_id, r.note,
           r.day_of_month, r.enabled, r.last_run_month,
           c.name, c.icon, a.name
    FROM recurring_rules r
    LEFT JOIN categories c ON r.category_id = c.id
    LEFT JOIN accounts a ON r.account_id = a.id
";

#[tauri::command]
pub fn list_recurring_rules(state: State<AppState>) -> Result<Vec<RecurringRule>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(&format!("{RULE_SELECT} ORDER BY r.id"))
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], map_rule)
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_recurring_rule(state: State<AppState>, input: NewRecurringRule) -> Result<RecurringRule, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let note = input.note.unwrap_or_default();
    let day = input.day_of_month.clamp(1, 28);

    conn.execute(
        "INSERT INTO recurring_rules (type, amount, category_id, account_id, note, day_of_month)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            input.r#type,
            input.amount,
            input.category_id,
            input.account_id,
            note,
            day
        ],
    )
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    conn.query_row(
        &format!("{RULE_SELECT} WHERE r.id = ?1"),
        params![id],
        map_rule,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_recurring_rule(
    state: State<AppState>,
    id: i64,
    input: UpdateRecurringRule,
) -> Result<RecurringRule, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let note = input.note.unwrap_or_default();
    let day = input.day_of_month.clamp(1, 28);

    conn.execute(
        "UPDATE recurring_rules SET type = ?1, amount = ?2, category_id = ?3, account_id = ?4,
         note = ?5, day_of_month = ?6 WHERE id = ?7",
        params![
            input.r#type,
            input.amount,
            input.category_id,
            input.account_id,
            note,
            day,
            id
        ],
    )
    .map_err(|e| e.to_string())?;

    conn.query_row(
        &format!("{RULE_SELECT} WHERE r.id = ?1"),
        params![id],
        map_rule,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn toggle_recurring_rule(state: State<AppState>, id: i64, enabled: bool) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE recurring_rules SET enabled = ?1 WHERE id = ?2",
        params![if enabled { 1 } else { 0 }, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_recurring_rule(state: State<AppState>, id: i64) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM recurring_rules WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn process_recurring_rules(state: &AppState) -> Result<usize, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let month = current_month();
    let day: u32 = today()
        .split('-')
        .nth(2)
        .and_then(|d| d.parse().ok())
        .unwrap_or(1);

    let mut stmt = conn
        .prepare(
            "SELECT id, type, amount, category_id, account_id, note, day_of_month, last_run_month
             FROM recurring_rules WHERE enabled = 1",
        )
        .map_err(|e| e.to_string())?;

    let rules: Vec<(i64, String, f64, Option<i64>, Option<i64>, String, i64, Option<String>)> = stmt
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut created = 0usize;
    for (id, tx_type, amount, category_id, account_id, note, rule_day, last_run) in rules {
        if last_run.as_deref() == Some(month.as_str()) {
            continue;
        }
        if day < rule_day as u32 {
            continue;
        }

        let tx_date = format!("{month}-{:02}", rule_day);

        conn.execute(
            "INSERT INTO transactions (type, amount, category_id, account_id, note, transaction_date)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![tx_type, amount, category_id, account_id, note, tx_date],
        )
        .map_err(|e| e.to_string())?;

        if let Some(acc_id) = account_id {
            let delta = if tx_type == "income" { amount } else { -amount };
            conn.execute(
                "UPDATE accounts SET balance = balance + ?1 WHERE id = ?2",
                params![delta, acc_id],
            )
            .map_err(|e| e.to_string())?;
        }

        conn.execute(
            "UPDATE recurring_rules SET last_run_month = ?1 WHERE id = ?2",
            params![month, id],
        )
        .map_err(|e| e.to_string())?;

        created += 1;
    }

    if created > 0 {
        conn.execute(
            "INSERT INTO settings (key, value) VALUES ('last_recurring_run', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![now_local()],
        )
        .ok();
    }

    Ok(created)
}
