use crate::db::{get_setting, now_local};
use crate::models::Settings;
use crate::AppState;
use rusqlite::params;
use tauri::State;

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Result<Settings, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    Ok(Settings {
        quote_update_interval: get_setting(&conn, "quote_update_interval")
            .and_then(|v| v.parse().ok())
            .unwrap_or(30),
        quote_update_enabled: get_setting(&conn, "quote_update_enabled")
            .map(|v| v == "true")
            .unwrap_or(true),
        refresh_on_startup: get_setting(&conn, "refresh_on_startup")
            .map(|v| v == "true")
            .unwrap_or(true),
        currency: get_setting(&conn, "currency").unwrap_or_else(|| "CNY".to_string()),
        ai_enabled: get_setting(&conn, "ai_enabled")
            .map(|v| v == "true")
            .unwrap_or(false),
        ai_api_key: get_setting(&conn, "ai_api_key").unwrap_or_default(),
        ai_api_base: get_setting(&conn, "ai_api_base")
            .unwrap_or_else(|| "https://v2.pincc.ai/v1".to_string()),
        ai_model: get_setting(&conn, "ai_model").unwrap_or_else(|| "gpt-4o-mini".to_string()),
    })
}

#[tauri::command]
pub fn update_settings(state: State<AppState>, settings: Settings) -> Result<Settings, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let entries = [
        (
            "quote_update_interval",
            settings.quote_update_interval.to_string(),
        ),
        (
            "quote_update_enabled",
            settings.quote_update_enabled.to_string(),
        ),
        (
            "refresh_on_startup",
            settings.refresh_on_startup.to_string(),
        ),
        ("currency", settings.currency.clone()),
        ("ai_enabled", settings.ai_enabled.to_string()),
        ("ai_api_key", settings.ai_api_key.clone()),
        ("ai_api_base", settings.ai_api_base.clone()),
        ("ai_model", settings.ai_model.clone()),
    ];

    for (key, value) in entries {
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )
        .map_err(|e| e.to_string())?;
    }

    conn.execute(
        "INSERT INTO settings (key, value) VALUES ('settings_updated_at', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![now_local()],
    )
    .ok();

    drop(conn);
    get_settings(state)
}

#[tauri::command]
pub fn get_last_sync_at(state: State<AppState>) -> Result<Option<String>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    Ok(get_setting(&conn, "last_sync_at"))
}

pub fn should_refresh_on_startup(state: &AppState) -> bool {
    let conn = match state.db.lock() {
        Ok(c) => c,
        Err(_) => return true,
    };
    get_setting(&conn, "refresh_on_startup")
        .map(|v| v == "true")
        .unwrap_or(true)
}
