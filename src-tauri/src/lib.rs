pub mod commands;
mod db;
pub mod models;
pub mod services;

use commands::holdings::refresh_all_quotes;
use commands::recurring::process_recurring_rules;
use commands::settings::should_refresh_on_startup;
use db::init_db;
use services::scheduler::start_quote_scheduler;
use std::str::FromStr;
use std::sync::Mutex;
use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

pub struct AppState {
    pub db: Mutex<rusqlite::Connection>,
}

fn register_global_shortcuts(app: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let shortcut = Shortcut::from_str("CommandOrControl+N")?;
    app.global_shortcut()
        .on_shortcut(shortcut, |app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                let _ = app.emit("quick-add-transaction", ());
            }
        })?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let conn = init_db(app.handle()).map_err(|e| e.to_string())?;
            app.manage(AppState {
                db: Mutex::new(conn),
            });
            start_quote_scheduler(app.handle().clone());

            if let Err(err) = register_global_shortcuts(app.handle()) {
                eprintln!("全局快捷键注册失败: {err}");
            }

            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let state = handle.state::<AppState>();
                let _ = process_recurring_rules(&state);
                if should_refresh_on_startup(&state) {
                    let _ = refresh_all_quotes(&state).await;
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_accounts,
            commands::add_account,
            commands::update_account,
            commands::delete_account,
            commands::list_categories,
            commands::add_category,
            commands::update_category,
            commands::delete_category,
            commands::list_transactions,
            commands::add_transaction,
            commands::update_transaction,
            commands::delete_transaction,
            commands::add_transfer,
            commands::get_dashboard,
            commands::get_monthly_stats,
            commands::get_category_stats,
            commands::get_portfolio_history,
            commands::list_budgets,
            commands::set_budget,
            commands::delete_budget,
            commands::get_budget_alerts,
            commands::list_recurring_rules,
            commands::add_recurring_rule,
            commands::update_recurring_rule,
            commands::toggle_recurring_rule,
            commands::delete_recurring_rule,
            commands::list_holdings,
            commands::add_holding,
            commands::lookup_holding_name,
            commands::update_holding,
            commands::delete_holding,
            commands::get_price_history,
            commands::get_kline_data,
            commands::refresh_quotes,
            commands::list_quant_watchlist,
            commands::add_quant_watchlist,
            commands::update_quant_watchlist,
            commands::delete_quant_watchlist,
            commands::list_quant_targets,
            commands::list_quant_signals,
            commands::get_quant_dashboard,
            commands::refresh_quant_signals,
            commands::update_quant_strategy_settings,
            commands::get_settings,
            commands::update_settings,
            commands::get_app_lock_status,
            commands::setup_app_lock,
            commands::verify_app_lock,
            commands::change_app_lock_password,
            commands::disable_app_lock,
            commands::get_last_sync_at,
            commands::get_db_path,
            commands::export_data,
            commands::import_data,
            commands::parse_transaction_nl_command,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
