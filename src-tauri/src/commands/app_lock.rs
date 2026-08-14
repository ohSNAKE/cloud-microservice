use crate::db::get_setting;
use crate::models::AppLockStatus;
use crate::AppState;
use bcrypt::{hash, verify, DEFAULT_COST};
use rusqlite::params;
use tauri::State;

const KEY_ENABLED: &str = "app_lock_enabled";
const KEY_HASH: &str = "app_lock_hash";

fn save_setting(conn: &rusqlite::Connection, key: &str, value: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn password_hash(password: &str) -> Result<String, String> {
    if password.trim().len() < 4 {
        return Err("密码至少 4 位".to_string());
    }
    hash(password, DEFAULT_COST).map_err(|e| e.to_string())
}

fn verify_stored_password(conn: &rusqlite::Connection, password: &str) -> Result<(), String> {
    let stored = get_setting(conn, KEY_HASH).ok_or("尚未设置应用锁密码")?;
    let ok = verify(password, &stored).map_err(|e| e.to_string())?;
    if ok {
        Ok(())
    } else {
        Err("密码错误".to_string())
    }
}

#[tauri::command]
pub fn get_app_lock_status(state: State<AppState>) -> Result<AppLockStatus, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let enabled = get_setting(&conn, KEY_ENABLED)
        .map(|v| v == "true")
        .unwrap_or(false);
    let configured = get_setting(&conn, KEY_HASH)
        .map(|v| !v.is_empty())
        .unwrap_or(false);
    Ok(AppLockStatus {
        enabled,
        configured,
    })
}

#[tauri::command]
pub fn setup_app_lock(state: State<AppState>, password: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let hashed = password_hash(&password)?;
    save_setting(&conn, KEY_HASH, &hashed)?;
    save_setting(&conn, KEY_ENABLED, "true")?;
    Ok(())
}

#[tauri::command]
pub fn verify_app_lock(state: State<AppState>, password: String) -> Result<bool, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    verify_stored_password(&conn, &password).map(|_| true)
}

#[tauri::command]
pub fn change_app_lock_password(
    state: State<AppState>,
    old_password: String,
    new_password: String,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    verify_stored_password(&conn, &old_password)?;
    let hashed = password_hash(&new_password)?;
    save_setting(&conn, KEY_HASH, &hashed)?;
    Ok(())
}

#[tauri::command]
pub fn disable_app_lock(state: State<AppState>, password: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    verify_stored_password(&conn, &password)?;
    save_setting(&conn, KEY_ENABLED, "false")?;
    conn.execute("DELETE FROM settings WHERE key = ?1", params![KEY_HASH])
        .map_err(|e| e.to_string())?;
    Ok(())
}
