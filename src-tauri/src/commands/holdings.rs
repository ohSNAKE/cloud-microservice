use crate::db::{get_setting, now_local, today};
use crate::models::{Holding, KlineData, NewHolding, PricePoint, QuoteRefreshResult};
use crate::services::quote::{
    fetch_fund_kline, fetch_fund_price, fetch_stock_kline, fetch_stock_price, lookup_stock_name,
};
use crate::AppState;
use rusqlite::params;
use tauri::State;

fn enrich_holding(row: &rusqlite::Row) -> rusqlite::Result<Holding> {
    let quantity: f64 = row.get(4)?;
    let cost_price: f64 = row.get(5)?;
    let current_price: f64 = row.get(6)?;
    let cost_value = quantity * cost_price;
    let market_value = quantity * current_price;
    let profit = market_value - cost_value;
    let profit_rate = if cost_value > 0.0 {
        (profit / cost_value) * 100.0
    } else {
        0.0
    };

    Ok(Holding {
        id: row.get(0)?,
        code: row.get(1)?,
        name: row.get(2)?,
        r#type: row.get(3)?,
        quantity,
        cost_price,
        current_price,
        market: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
        market_value,
        cost_value,
        profit,
        profit_rate,
    })
}

#[tauri::command]
pub fn list_holdings(state: State<AppState>) -> Result<Vec<Holding>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, code, name, type, quantity, cost_price, current_price, market, created_at, updated_at
             FROM holdings ORDER BY id DESC",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], enrich_holding)
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_holding(state: State<'_, AppState>, mut input: NewHolding) -> Result<Holding, String> {
    if input.name.trim().is_empty() && input.r#type == "stock" {
        input.name = lookup_stock_name(&input.code).await.unwrap_or_else(|_| input.code.clone());
    }
    if input.name.trim().is_empty() {
        input.name = input.code.clone();
    }

    let market = input.market.unwrap_or_else(|| "cn".to_string());
    let id = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO holdings (code, name, type, quantity, cost_price, current_price, market)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5, ?6)",
            params![
                input.code,
                input.name,
                input.r#type,
                input.quantity,
                input.cost_price,
                market
            ],
        )
        .map_err(|e| e.to_string())?;
        conn.last_insert_rowid()
    };

    refresh_all_quotes(&state).await.ok();

    list_holdings(state)?
        .into_iter()
        .find(|h| h.id == id)
        .ok_or_else(|| "创建持仓失败".to_string())
}

#[tauri::command]
pub fn update_holding(
    state: State<AppState>,
    id: i64,
    quantity: f64,
    cost_price: f64,
) -> Result<Holding, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE holdings SET quantity = ?1, cost_price = ?2, updated_at = ?3 WHERE id = ?4",
        params![quantity, cost_price, now_local(), id],
    )
    .map_err(|e| e.to_string())?;
    drop(conn);

    list_holdings(state)?
        .into_iter()
        .find(|h| h.id == id)
        .ok_or_else(|| "持仓不存在".to_string())
}

#[tauri::command]
pub fn delete_holding(state: State<AppState>, id: i64) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM holdings WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_price_history(
    state: State<AppState>,
    holding_id: i64,
    days: Option<i64>,
) -> Result<Vec<PricePoint>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let limit = days.unwrap_or(30);

    let mut stmt = conn
        .prepare(
            "SELECT price, recorded_at FROM price_history
             WHERE holding_id = ?1
             ORDER BY recorded_at DESC
             LIMIT ?2",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(params![holding_id, limit], |row| {
            Ok(PricePoint {
                price: row.get(0)?,
                recorded_at: row.get(1)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut points: Vec<PricePoint> = rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
    points.reverse();
    Ok(points)
}

#[tauri::command]
pub async fn get_kline_data(
    code: String,
    kind: String,
    period: Option<String>,
    limit: Option<i64>,
) -> Result<KlineData, String> {
    let period = period.unwrap_or_else(|| "day".to_string());
    let limit = limit.unwrap_or(120);

    if kind == "fund" {
        let bars = fetch_fund_kline(&code, limit).await?;
        return Ok(KlineData {
            bars,
            chart_type: "line".to_string(),
        });
    }

    let bars = fetch_stock_kline(&code, &period, limit).await?;
    Ok(KlineData {
        bars,
        chart_type: "candlestick".to_string(),
    })
}

#[tauri::command]
pub async fn refresh_quotes(state: State<'_, AppState>) -> Result<QuoteRefreshResult, String> {
    refresh_all_quotes(&state).await
}

pub async fn refresh_all_quotes(state: &AppState) -> Result<QuoteRefreshResult, String> {
    let holdings: Vec<(i64, String, String)> = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT id, code, type FROM holdings")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?
    };

    let mut updated = 0usize;
    let mut failed = 0usize;
    let mut failed_codes = Vec::new();
    let recorded_at = format!("{} {}", today(), chrono::Local::now().format("%H:%M:%S"));

    for (id, code, kind) in holdings {
        let price_result = if kind == "fund" {
            fetch_fund_price(&code).await
        } else {
            fetch_stock_price(&code).await
        };

        match price_result {
            Ok(price) if price > 0.0 => {
                let conn = state.db.lock().map_err(|e| e.to_string())?;
                conn.execute(
                    "UPDATE holdings SET current_price = ?1, updated_at = ?2 WHERE id = ?3",
                    params![price, now_local(), id],
                )
                .map_err(|e| e.to_string())?;
                conn.execute(
                    "INSERT INTO price_history (holding_id, price, recorded_at) VALUES (?1, ?2, ?3)",
                    params![id, price, recorded_at],
                )
                .ok();
                updated += 1;
            }
            _ => {
                failed += 1;
                failed_codes.push(code);
            }
        }
    }

    {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO settings (key, value) VALUES ('last_sync_at', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![now_local()],
        )
        .ok();
    }

    Ok(QuoteRefreshResult {
        updated,
        failed,
        message: format!("已更新 {updated} 条，失败 {failed} 条"),
        failed_codes,
    })
}

pub fn is_quote_enabled(state: &AppState) -> bool {
    let conn = match state.db.lock() {
        Ok(c) => c,
        Err(_) => return false,
    };
    get_setting(&conn, "quote_update_enabled")
        .map(|v| v == "true")
        .unwrap_or(true)
}

pub fn quote_interval_minutes(state: &AppState) -> u64 {
    let conn = match state.db.lock() {
        Ok(c) => c,
        Err(_) => return 30,
    };
    get_setting(&conn, "quote_update_interval")
        .and_then(|v| v.parse().ok())
        .unwrap_or(30)
}
