use crate::commands::holdings::{is_quote_enabled, quote_interval_minutes, refresh_all_quotes};
use crate::AppState;
use std::time::Duration;
use tauri::Manager;

pub fn start_quote_scheduler(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            let state = app.state::<AppState>();
            let interval = quote_interval_minutes(&*state);
            tokio::time::sleep(Duration::from_secs(interval * 60)).await;

            if !is_quote_enabled(&*state) {
                continue;
            }

            if let Err(err) = refresh_all_quotes(&*state).await {
                eprintln!("定时行情更新失败: {err}");
            }
        }
    });
}
