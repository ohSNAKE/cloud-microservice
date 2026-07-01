mod db;
pub mod models;
pub mod commands;
pub mod services;

use db::init_db;
use services::scheduler::start_quote_scheduler;
use std::sync::Mutex;
use tauri::Manager;

pub struct AppState {
    pub db: Mutex<rusqlite::Connection>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let conn = init_db(app.handle()).map_err(|e| e.to_string())?;
            app.manage(AppState {
                db: Mutex::new(conn),
            });
            start_quote_scheduler(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_accounts,
            commands::list_categories,
            commands::list_transactions,
            commands::add_transaction,
            commands::delete_transaction,
            commands::get_dashboard,
            commands::get_monthly_stats,
            commands::list_holdings,
            commands::add_holding,
            commands::update_holding,
            commands::delete_holding,
            commands::get_price_history,
            commands::get_kline_data,
            commands::refresh_quotes,
            commands::get_settings,
            commands::update_settings,
            commands::get_last_sync_at,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
