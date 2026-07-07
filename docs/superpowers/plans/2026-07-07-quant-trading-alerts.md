# Quant Trading Alerts Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a stock-only quantitative alert page that analyzes holdings and watchlist stocks with MA5/MA20 trend filtering, strict realtime quote validation, automatic grid triggers, 5-minute deduped alerts, in-app history, and desktop notifications.

**Architecture:** Keep quant logic backend-owned in focused Rust service modules, exposed through Tauri commands. The frontend owns page polling lifecycle only; each refresh asks the backend to recompute targets, trend, grid, strict quote status, current output state, and newly generated signals. SQLite persists watchlist, strategy settings, and signal history without mutating holdings, accounts, transactions, or price history.

**Tech Stack:** Tauri 2, Rust, rusqlite, ureq, chrono, React 19, TypeScript, Ant Design, `@tauri-apps/plugin-notification` / `tauri-plugin-notification`.

---

Spec: `docs/superpowers/specs/2026-07-07-quant-trading-alerts-design.md`

## File Structure

- Modify `src-tauri/src/models.rs`: add quant DTOs, export/import row structs, and `#[serde(default)]` quant arrays on `ExportPayload`.
- Modify `src-tauri/src/db.rs`: create/migrate `quant_watchlist`, `quant_strategy_settings`, and `quant_signals` tables and indexes.
- Create `src-tauri/src/services/quant.rs`: pure quant calculations, target merging helpers, strict trading-time validation, dedupe helpers, and unit tests.
- Modify `src-tauri/src/services/quote.rs`: add read-only stock realtime quote fetch with exchange timestamp or market-status validation; do not write finance tables.
- Modify `src-tauri/src/services/mod.rs`: export the new quant service module.
- Create `src-tauri/src/commands/quant.rs`: Tauri commands for watchlist, strategy settings, dashboard refresh, and signal history.
- In `src-tauri/src/commands/quant.rs`, keep command wrappers thin and put testable database/runtime helpers in regular functions such as `merge_quant_targets`, `list_quant_signals_from_conn`, and `refresh_quant_signals_with_provider`.
- Modify `src-tauri/src/commands/mod.rs`: re-export quant commands.
- Modify `src-tauri/src/lib.rs`: register quant commands and notification plugin.
- Modify `src-tauri/Cargo.toml`: add `tauri-plugin-notification = "2"`.
- Modify `src-tauri/Cargo.lock`: updated by Cargo after adding `tauri-plugin-notification`.
- Modify `package.json` and `package-lock.json`: add `@tauri-apps/plugin-notification`.
- Modify `src-tauri/capabilities/default.json`: add notification permissions.
- Modify `src-tauri/src/commands/data.rs`: export/import quant tables and clear them during import.
- Modify `src/types/index.ts`: add quant frontend types.
- Modify `src/api/index.ts`: add quant API wrappers.
- Create `src/pages/QuantAlerts.tsx`: quant alert page with status, signal board, watchlist management, and history.
- Create `src/utils/quantNotifications.ts`: small frontend helper that requests notification permission only when generated signals exist and sends notifications only for eligible `generated_signals`.
- Modify `src/App.tsx`: add `量化提醒` navigation route.
- Modify `src/styles/global.css`: add quant page/card styles following existing app patterns.

## Chunk 1: Backend Data Model And Migrations

### Task 1: Add Quant Models And Export Payload Fields

**Files:**
- Modify: `src-tauri/src/models.rs:277-308`

- [ ] **Step 1: Write model definitions before implementation use**

Add these Rust structs near `ExportPayload`:

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QuantWatchlistItem {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub market: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NewQuantWatchlistItem {
    pub code: String,
    pub name: Option<String>,
    pub market: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QuantWatchlistItemUpdate {
    pub name: String,
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QuantStrategySettingsUpdate {
    pub enabled: bool,
    pub desktop_notification_enabled: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QuantGridZone {
    pub zone: String,
    pub direction: String,
    pub lower: f64,
    pub upper: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QuantSignal {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub market: String,
    pub direction: String,
    pub trigger_price: f64,
    pub trend_state: String,
    pub source: String,
    pub trigger_zone: String,
    pub triggered_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QuantGeneratedSignal {
    pub signal: QuantSignal,
    pub desktop_notification_enabled: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QuantTarget {
    pub code: String,
    pub name: String,
    pub market: String,
    pub source: String,
    pub enabled: bool,
    pub desktop_notification_enabled: bool,
    pub current_price: Option<f64>,
    pub quote_fetched_at: Option<String>,
    pub trend_state: String,
    pub output_state: String,
    pub current_trigger_zone: Option<String>,
    pub ma_short: Option<f64>,
    pub ma_long: Option<f64>,
    pub grid_zones: Vec<QuantGridZone>,
    pub latest_signal: Option<QuantSignal>,
    pub last_error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QuantDashboard {
    pub is_trading_time: bool,
    pub next_refresh_at: Option<String>,
    pub targets: Vec<QuantTarget>,
    pub recent_signals: Vec<QuantSignal>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QuantRefreshResult {
    pub dashboard: QuantDashboard,
    pub generated_signals: Vec<QuantGeneratedSignal>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct QuantSignalFilter {
    pub code: Option<String>,
    pub direction: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QuantWatchlistRow {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub market: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QuantStrategySettingsRow {
    pub id: i64,
    pub code: String,
    pub market: String,
    pub ma_short: i64,
    pub ma_long: i64,
    pub grid_lookback_days: i64,
    pub poll_interval_seconds: i64,
    pub desktop_notification_enabled: bool,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QuantSignalRow {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub market: String,
    pub direction: String,
    pub trigger_price: f64,
    pub trend_state: String,
    pub source: String,
    pub trigger_zone: String,
    pub dedupe_key: String,
    pub triggered_at: String,
    pub created_at: String,
}
```

- [ ] **Step 2: Add defaulted quant arrays to export payload**

Update `ExportPayload`:

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExportPayload {
    pub version: String,
    pub exported_at: String,
    pub accounts: Vec<Account>,
    pub categories: Vec<Category>,
    pub transactions: Vec<TransactionRow>,
    pub holdings: Vec<HoldingRow>,
    pub settings: Vec<(String, String)>,
    #[serde(default)]
    pub quant_watchlist: Vec<QuantWatchlistRow>,
    #[serde(default)]
    pub quant_strategy_settings: Vec<QuantStrategySettingsRow>,
    #[serde(default)]
    pub quant_signals: Vec<QuantSignalRow>,
}
```

- [ ] **Step 3: Run backend compile check**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`

Expected: compile failures only for missing construction fields in `ExportPayload`, which will be fixed in Task 3. No syntax errors in new structs.

### Task 2: Add SQLite Tables And Indexes

**Files:**
- Modify: `src-tauri/src/db.rs:15-103`
- Modify: `src-tauri/src/db.rs:127-152`

- [ ] **Step 1: Add table creation SQL to initial schema**

In `init_db` execute batch, add:

```sql
CREATE TABLE IF NOT EXISTS quant_watchlist (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    code TEXT NOT NULL,
    name TEXT NOT NULL,
    market TEXT NOT NULL DEFAULT 'cn',
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
    UNIQUE(code, market)
);

CREATE TABLE IF NOT EXISTS quant_strategy_settings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    code TEXT NOT NULL,
    market TEXT NOT NULL DEFAULT 'cn',
    ma_short INTEGER NOT NULL DEFAULT 5,
    ma_long INTEGER NOT NULL DEFAULT 20,
    grid_lookback_days INTEGER NOT NULL DEFAULT 20,
    poll_interval_seconds INTEGER NOT NULL DEFAULT 60,
    desktop_notification_enabled INTEGER NOT NULL DEFAULT 1,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
    UNIQUE(code, market)
);

CREATE TABLE IF NOT EXISTS quant_signals (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    code TEXT NOT NULL,
    name TEXT NOT NULL,
    market TEXT NOT NULL DEFAULT 'cn',
    direction TEXT NOT NULL,
    trigger_price REAL NOT NULL,
    trend_state TEXT NOT NULL,
    source TEXT NOT NULL,
    trigger_zone TEXT NOT NULL,
    dedupe_key TEXT NOT NULL,
    triggered_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
);

CREATE INDEX IF NOT EXISTS idx_quant_signals_code_time ON quant_signals(code, triggered_at);
CREATE INDEX IF NOT EXISTS idx_quant_signals_dedupe_time ON quant_signals(dedupe_key, triggered_at);
```

- [ ] **Step 2: Add same table/index SQL to `migrate`**

Add the same `CREATE TABLE IF NOT EXISTS` and `CREATE INDEX IF NOT EXISTS` SQL in `migrate` so existing users get the tables.

- [ ] **Step 3: Run backend compile check**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`

Expected: still only missing `ExportPayload` fields until Task 3, or PASS if Task 1 construction was already updated.

### Task 3: Include Quant Tables In Export And Import

**Files:**
- Modify: `src-tauri/src/commands/data.rs:1-190`
- Modify: `src-tauri/src/models.rs:277-286`

- [ ] **Step 1: Write failing import/export compatibility tests first**

Before implementing export/import support, add the tests described in Task 4:

- `export_payload_accepts_missing_quant_fields`
- `old_backup_import_clears_stale_quant_rows`
- `export_import_preserves_quant_rows_with_ids`

Run: `cargo test --manifest-path src-tauri/Cargo.toml quant -- --nocapture`

Expected: FAIL because export/import helpers and quant payload fields are not implemented yet.

- [ ] **Step 2: Update imports**

In `data.rs`, import `QuantSignalRow`, `QuantStrategySettingsRow`, and `QuantWatchlistRow`.

- [ ] **Step 3: Export quant arrays**

Add SELECT queries for each quant table and populate `ExportPayload` fields:

```rust
let quant_watchlist: Vec<QuantWatchlistRow> = conn
    .prepare("SELECT id, code, name, market, enabled, created_at, updated_at FROM quant_watchlist ORDER BY id")?
    .query_map([], |row| Ok(QuantWatchlistRow {
        id: row.get(0)?,
        code: row.get(1)?,
        name: row.get(2)?,
        market: row.get(3)?,
        enabled: row.get::<_, i64>(4)? != 0,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    }))?
    .collect::<Result<Vec<_>, _>>()?;
```

Repeat similarly for `quant_strategy_settings` and `quant_signals`. Map boolean integer columns to `bool` in row structs.

- [ ] **Step 4: Update payload construction**

Add:

```rust
quant_watchlist,
quant_strategy_settings,
quant_signals,
```

- [ ] **Step 5: Clear quant tables during import**

In `import_data`, extend the delete batch before core table deletes:

```sql
DELETE FROM quant_signals;
DELETE FROM quant_strategy_settings;
DELETE FROM quant_watchlist;
```

- [ ] **Step 6: Insert imported quant rows**

After settings insertion, insert all `payload.quant_watchlist`, `payload.quant_strategy_settings`, and `payload.quant_signals` rows using explicit IDs. Convert booleans with `if row.enabled { 1 } else { 0 }`.

- [ ] **Step 7: Run compile check**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`

Expected: PASS after `ExportPayload` construction includes the new quant fields.

### Task 4: Verify Backend Tests For Payload Compatibility

**Files:**
- Modify: `src-tauri/src/models.rs`

- [ ] **Step 1: Ensure serde test exists**

Add a `#[cfg(test)] mod tests` near the end of `models.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_payload_accepts_missing_quant_fields() {
        let json = r#"{
            "version":"1.0",
            "exported_at":"2026-07-07 10:00:00",
            "accounts":[],
            "categories":[],
            "transactions":[],
            "holdings":[],
            "settings":[]
        }"#;

        let payload: ExportPayload = serde_json::from_str(json).unwrap();
        assert!(payload.quant_watchlist.is_empty());
        assert!(payload.quant_strategy_settings.is_empty());
        assert!(payload.quant_signals.is_empty());
    }
}
```

- [ ] **Step 2: Run the serde test**

Run: `cargo test --manifest-path src-tauri/Cargo.toml export_payload_accepts_missing_quant_fields -- --nocapture`

Expected: PASS.

- [ ] **Step 3: Ensure import/export compatibility tests exist**

Add tests in `src-tauri/src/commands/data.rs` or a focused backend test module for:

```rust
#[test]
fn old_backup_import_clears_stale_quant_rows() {
    let conn = test_conn_with_schema();
    conn.execute("INSERT INTO quant_watchlist (code, name, enabled) VALUES ('600000', '浦发银行', 1)", []).unwrap();
    conn.execute("INSERT INTO quant_strategy_settings (code, enabled) VALUES ('600000', 1)", []).unwrap();
    conn.execute("INSERT INTO quant_signals (code, name, direction, trigger_price, trend_state, source, trigger_zone, dedupe_key, triggered_at) VALUES ('600000', '浦发银行', 'buy_attention', 10.0, 'neutral', 'auto_grid', 'buy_1', 'cn:600000:buy_attention:buy_1', '2026-07-07 10:00:00')", []).unwrap();

    let old_backup = r#"{"version":"1.0","exported_at":"2026-07-07 10:00:00","accounts":[],"categories":[],"transactions":[],"holdings":[],"settings":[]}"#;
    import_payload_json(&conn, old_backup).unwrap();

    assert_eq!(count_rows(&conn, "quant_watchlist"), 0);
    assert_eq!(count_rows(&conn, "quant_strategy_settings"), 0);
    assert_eq!(count_rows(&conn, "quant_signals"), 0);
}

#[test]
fn export_import_preserves_quant_rows_with_ids() {
    let source = test_conn_with_schema();
    source.execute("INSERT INTO quant_watchlist (id, code, name, market, enabled, created_at, updated_at) VALUES (7, '600000', '浦发银行', 'cn', 1, '2026-07-07 09:00:00', '2026-07-07 09:00:00')", []).unwrap();
    source.execute("INSERT INTO quant_strategy_settings (id, code, market, enabled, desktop_notification_enabled) VALUES (8, '600000', 'cn', 1, 0)", []).unwrap();
    source.execute("INSERT INTO quant_signals (id, code, name, market, direction, trigger_price, trend_state, source, trigger_zone, dedupe_key, triggered_at, created_at) VALUES (9, '600000', '浦发银行', 'cn', 'buy_attention', 12.34, 'neutral', 'auto_grid', 'buy_1', 'cn:600000:buy_attention:buy_1', '2026-07-07 10:00:00', '2026-07-07 10:00:01')", []).unwrap();
    let json = export_payload_json(&source).unwrap();

    let target = test_conn_with_schema();
    import_payload_json(&target, &json).unwrap();

    assert_eq!(target.query_row::<i64, _, _>("SELECT id FROM quant_watchlist WHERE code = '600000'", [], |row| row.get(0)).unwrap(), 7);
    assert_eq!(target.query_row::<i64, _, _>("SELECT desktop_notification_enabled FROM quant_strategy_settings WHERE code = '600000'", [], |row| row.get(0)).unwrap(), 0);
    let signal: (i64, String, String) = target.query_row("SELECT id, dedupe_key, triggered_at FROM quant_signals WHERE code = '600000'", [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).unwrap();
    assert_eq!(signal, (9, "cn:600000:buy_attention:buy_1".to_string(), "2026-07-07 10:00:00".to_string()));
}
```

Use an in-memory `rusqlite::Connection` and helper functions factored from `export_data` / `import_data` if needed. The old-backup test must insert stale quant rows first, import a payload without quant arrays, and assert `quant_watchlist`, `quant_strategy_settings`, and `quant_signals` are empty afterward.

- [ ] **Step 4: Run compatibility tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml quant -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Commit chunk 1**

Run only when commits are explicitly requested:

```bash
git add src-tauri/src/models.rs src-tauri/src/db.rs src-tauri/src/commands/data.rs
git commit -m "feat(quant): add storage schema"
```

## Chunk 2: Quant Service And Strict Quote Validation

### Task 5: Add Pure Quant Calculation Service

**Files:**
- Create: `src-tauri/src/services/quant.rs`
- Modify: `src-tauri/src/services/mod.rs`

- [ ] **Step 1: Write failing unit tests first**

Create `src-tauri/src/services/quant.rs` with test module containing:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn bars(closes: &[f64]) -> Vec<crate::models::KlineBar> {
        closes.iter().enumerate().map(|(i, close)| crate::models::KlineBar {
            date: format!("2026-06-{:02}", i + 1),
            open: *close,
            close: *close,
            low: close - 0.5,
            high: close + 0.5,
            volume: 1000.0,
            change_pct: 0.0,
        }).collect()
    }

    #[test]
    fn calculates_ma() {
        assert_eq!(calculate_ma(&[1.0, 2.0, 3.0, 4.0, 5.0], 5), Some(3.0));
        assert_eq!(calculate_ma(&[1.0, 2.0], 5), None);
    }

    #[test]
    fn classifies_trend() {
        let up = bars(&(1..=20).map(|v| v as f64).collect::<Vec<_>>());
        let trend = classify_trend(&up, 5, 20);
        assert_eq!(trend.state, "bullish");
        assert!(trend.ma_short.unwrap() > trend.ma_long.unwrap());
    }

    #[test]
    fn generates_grid_zones() {
        let input = bars(&(10..=29).map(|v| v as f64).collect::<Vec<_>>());
        let zones = generate_grid_zones(&input, 20).unwrap();
        assert_eq!(zones.len(), 4);
        assert_eq!(zones[0].zone, "buy_1");
        assert_eq!(zones[3].zone, "sell_2");
    }

    #[test]
    fn decides_signal_from_grid_and_trend() {
        let zones = vec![QuantGridZoneInternal { zone: "buy_1".into(), direction: "buy_attention".into(), lower: 9.0, upper: 10.0 }];
        let decision = decide_signal(9.5, "neutral", &zones);
        assert_eq!(decision.output_state, "buy_attention");
        assert_eq!(decision.current_trigger_zone.as_deref(), Some("buy_1"));
    }

    #[test]
    fn cooldown_key_includes_market() {
        assert_eq!(dedupe_key("600000", "cn", "buy_attention", "buy_1"), "cn:600000:buy_attention:buy_1");
    }

    #[test]
    fn insufficient_data_does_not_create_grid() {
        let input = bars(&[10.0, 11.0, 12.0]);
        assert!(generate_grid_zones(&input, 20).is_none());
        assert_eq!(classify_trend(&input, 5, 20).state, "insufficient_data");
    }

    #[test]
    fn grid_boundaries_match_spec() {
        let zones = vec![
            QuantGridZoneInternal { zone: "buy_1".into(), direction: "buy_attention".into(), lower: 9.0, upper: 10.0 },
            QuantGridZoneInternal { zone: "sell_1".into(), direction: "sell_attention".into(), lower: 10.0, upper: 11.0 },
        ];
        assert_eq!(decide_signal(9.0, "neutral", &zones).output_state, "buy_attention");
        assert_eq!(decide_signal(10.0, "neutral", &zones).output_state, "watch");
        assert_eq!(decide_signal(11.0, "neutral", &zones).output_state, "sell_attention");
    }

    #[test]
    fn trend_filter_suppresses_opposite_signals() {
        let zones = vec![QuantGridZoneInternal { zone: "buy_1".into(), direction: "buy_attention".into(), lower: 9.0, upper: 10.0 }];
        assert_eq!(decide_signal(9.5, "bearish", &zones).output_state, "watch");
    }

    #[test]
    fn excludes_unfinished_current_day_bar() {
        let mut input = bars(&(1..=21).map(|v| v as f64).collect::<Vec<_>>());
        input[20].date = "2026-07-07".to_string();
        let now = chrono::FixedOffset::east_opt(8 * 3600).unwrap()
            .with_ymd_and_hms(2026, 7, 7, 10, 0, 0).unwrap();

        let completed = completed_daily_bars(&input, now);

        assert_eq!(completed.len(), 20);
        assert_eq!(completed.last().unwrap().date, "2026-06-20");
    }

    #[test]
    fn excluding_current_day_still_keeps_twenty_completed_bars() {
        let mut input = bars(&(1..=21).map(|v| v as f64).collect::<Vec<_>>());
        input[20].date = "2026-07-07".to_string();
        let now = chrono::FixedOffset::east_opt(8 * 3600).unwrap()
            .with_ymd_and_hms(2026, 7, 7, 10, 0, 0).unwrap();

        let completed = completed_daily_bars(&input, now);
        let trend = classify_trend(&completed, 5, 20);
        let zones = generate_grid_zones(&completed, 20);

        assert_ne!(trend.state, "insufficient_data");
        assert!(zones.is_some());
    }
}
```

- [ ] **Step 2: Run tests to verify failure**

Run: `cargo test --manifest-path src-tauri/Cargo.toml services::quant -- --nocapture`

Expected: FAIL because functions and internal structs are not implemented.

- [ ] **Step 3: Implement pure functions**

Implement minimal public/internal functions:

```rust
pub struct TrendResult {
    pub state: String,
    pub ma_short: Option<f64>,
    pub ma_long: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct QuantGridZoneInternal {
    pub zone: String,
    pub direction: String,
    pub lower: f64,
    pub upper: f64,
}

pub struct SignalDecision {
    pub output_state: String,
    pub current_trigger_zone: Option<String>,
}

pub fn calculate_ma(closes: &[f64], period: usize) -> Option<f64> {
    if period == 0 || closes.len() < period {
        return None;
    }
    let slice = &closes[closes.len() - period..];
    Some(slice.iter().sum::<f64>() / period as f64)
}

pub fn classify_trend(bars: &[crate::models::KlineBar], ma_short: usize, ma_long: usize) -> TrendResult {
    let closes: Vec<f64> = bars.iter().map(|bar| bar.close).collect();
    let short = calculate_ma(&closes, ma_short);
    let long = calculate_ma(&closes, ma_long);
    let latest = closes.last().copied();
    let state = match (short, long, latest) {
        (Some(s), Some(l), Some(c)) if s > l && c >= s => "bullish",
        (Some(s), Some(l), Some(c)) if s < l && c <= s => "bearish",
        (Some(_), Some(_), Some(_)) => "neutral",
        _ => "insufficient_data",
    }.to_string();
    TrendResult { state, ma_short: short, ma_long: long }
}

pub fn generate_grid_zones(bars: &[crate::models::KlineBar], lookback_days: usize) -> Option<Vec<QuantGridZoneInternal>> {
    if bars.len() < lookback_days || lookback_days < 2 {
        return None;
    }
    let window = &bars[bars.len() - lookback_days..];
    let base = window.last()?.close;
    let high = window.iter().map(|bar| bar.high).fold(f64::MIN, f64::max);
    let low = window.iter().map(|bar| bar.low).fold(f64::MAX, f64::min);
    let range_step = (high - low) / 6.0;
    let moves: Vec<f64> = window.windows(2).map(|pair| (pair[1].close - pair[0].close).abs()).collect();
    let avg_abs_move = moves.iter().sum::<f64>() / moves.len() as f64;
    let step = range_step.max(avg_abs_move).max(base * 0.003);
    Some(vec![
        QuantGridZoneInternal { zone: "buy_1".into(), direction: "buy_attention".into(), lower: base - step, upper: base },
        QuantGridZoneInternal { zone: "buy_2".into(), direction: "buy_attention".into(), lower: base - 2.0 * step, upper: base - step },
        QuantGridZoneInternal { zone: "sell_1".into(), direction: "sell_attention".into(), lower: base, upper: base + step },
        QuantGridZoneInternal { zone: "sell_2".into(), direction: "sell_attention".into(), lower: base + step, upper: base + 2.0 * step },
    ])
}

pub fn decide_signal(price: f64, trend_state: &str, zones: &[QuantGridZoneInternal]) -> SignalDecision {
    for zone in zones {
        let in_zone = match zone.zone.as_str() {
            "buy_1" | "buy_2" => price >= zone.lower && price < zone.upper,
            "sell_1" | "sell_2" => price > zone.lower && price <= zone.upper,
            _ => false,
        };
        if !in_zone {
            continue;
        }
        if zone.direction == "buy_attention" && trend_state != "bearish" {
            return SignalDecision { output_state: zone.direction.clone(), current_trigger_zone: Some(zone.zone.clone()) };
        }
        if zone.direction == "sell_attention" && trend_state != "bullish" {
            return SignalDecision { output_state: zone.direction.clone(), current_trigger_zone: Some(zone.zone.clone()) };
        }
    }
    SignalDecision { output_state: "watch".into(), current_trigger_zone: None }
}

pub fn dedupe_key(code: &str, market: &str, direction: &str, trigger_zone: &str) -> String {
    format!("{market}:{code}:{direction}:{trigger_zone}")
}

pub fn completed_daily_bars(bars: &[crate::models::KlineBar], now: chrono::DateTime<chrono::FixedOffset>) -> Vec<crate::models::KlineBar> {
    let today = now.format("%Y-%m-%d").to_string();
    let market_closed = now.time() > chrono::NaiveTime::from_hms_opt(15, 0, 0).unwrap();
    bars.iter()
        .filter(|bar| market_closed || bar.date != today)
        .cloned()
        .collect()
}
```

Follow the spec formulas exactly.

- [ ] **Step 4: Export service module**

In `src-tauri/src/services/mod.rs`, add:

```rust
pub mod quant;
```

- [ ] **Step 5: Run tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml services::quant -- --nocapture`

Expected: PASS.

### Task 6: Add Read-Only Strict Realtime Quote Fetch

**Files:**
- Modify: `src-tauri/src/services/quote.rs`

- [ ] **Step 1: Add realtime quote DTO**

Add:

```rust
#[derive(Debug, Clone)]
pub struct RealtimeStockQuote {
    pub price: f64,
    pub exchange_timestamp: Option<i64>,
    pub market_status: Option<String>,
    pub quote_fetched_at: String,
}
```

- [ ] **Step 2: Add mandatory parser tests**

Factor response parsing into a testable function:

```rust
fn parse_realtime_stock_quote(code: &str, market: &str, text: &str) -> Result<RealtimeStockQuote, String> {
    #[derive(serde::Deserialize)]
    struct Response { data: Option<serde_json::Value> }

    let body: Response = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let data = body.data.ok_or_else(|| format!("无法获取 {market}:{code} 实时行情"))?;
    let raw_price = data.get("f43").and_then(|v| v.as_f64())
        .ok_or_else(|| format!("无法获取 {market}:{code} 实时价格"))?;
    let price = raw_price / 100.0;
    let exchange_timestamp = data.get("f124").and_then(|v| v.as_i64());
    let market_status = data.get("f292").map(|v| {
        v.as_str().map(str::to_string).unwrap_or_else(|| v.to_string())
    });
    if exchange_timestamp.is_none() && !quote_status_allows_strict_signal(market_status.as_deref()) {
        return Err(format!("{market}:{code} 缺少有效交易时间或交易状态"));
    }
    Ok(RealtimeStockQuote {
        price,
        exchange_timestamp,
        market_status,
        quote_fetched_at: crate::db::now_local(),
    })
}

fn quote_status_allows_strict_signal(market_status: Option<&str>) -> bool {
    matches!(market_status.map(str::trim), Some("5") | Some("交易中"))
}
```

Use EastMoney `push2.eastmoney.com/api/qt/stock/get` with fields that include at minimum latest price and exchange timestamp/market status. The planned field set is `f43,f58,f124,f292`, where `f43` is latest price in cents, `f124` is exchange update Unix timestamp, and `f292` is market status when available. The single strict rule is: if a quote includes `f124`, that timestamp must be in the current active trading session; an open `f292` value cannot override a stale or out-of-session timestamp. If no timestamp is present, explicit open-market status (`f292 = 5` or textual `交易中`) is acceptable. Reject all other responses. If live validation shows these fields are unavailable or unreliable, stop and pick a provider that has an exchange timestamp or market status; do not weaken strict mode.

Add parser tests:

```rust
#[test]
fn realtime_parser_rejects_missing_timestamp_and_status() {
    let json = r#"{"data":{"f43":1234,"f58":"浦发银行"}}"#;
    let err = parse_realtime_stock_quote("600000", "cn", json).unwrap_err();
    assert!(err.contains("时间") || err.contains("状态"));
}

#[test]
fn realtime_parser_rejects_missing_price() {
    let json = r#"{"data":{"f124":1783419000,"f292":5,"f58":"浦发银行"}}"#;
    let err = parse_realtime_stock_quote("600000", "cn", json).unwrap_err();
    assert!(err.contains("价格"));
}

#[test]
fn realtime_parser_accepts_price_and_exchange_timestamp() {
    let json = r#"{"data":{"f43":1234,"f124":1783419000,"f292":5,"f58":"浦发银行"}}"#;
    let quote = parse_realtime_stock_quote("600000", "cn", json).unwrap();
    assert_eq!(quote.price, 12.34);
    assert_eq!(quote.exchange_timestamp, Some(1783419000));
    assert!(!quote.quote_fetched_at.is_empty());
}

#[test]
fn realtime_parser_accepts_explicit_open_market_status_without_timestamp() {
    let json = r#"{"data":{"f43":1234,"f292":5,"f58":"浦发银行"}}"#;
    let quote = parse_realtime_stock_quote("600000", "cn", json).unwrap();
    assert_eq!(quote.market_status.as_deref(), Some("5"));
}

#[test]
fn realtime_parser_rejects_closed_market_status_without_timestamp() {
    let json = r#"{"data":{"f43":1234,"f292":0,"f58":"浦发银行"}}"#;
    assert!(parse_realtime_stock_quote("600000", "cn", json).is_err());
}

```

- [ ] **Step 3: Add strict fetch function**

Implement `pub async fn fetch_stock_realtime_quote(code: &str, market: &str) -> Result<RealtimeStockQuote, String>` using EastMoney quote fields that include price and quote time/status. The implementation must reject responses that include neither a usable exchange timestamp nor explicit open-market status.

Implementation note: use a separate URL from `refresh_quotes`; do not update `holdings` or `price_history`.

- [ ] **Step 4: Run parser tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml realtime_parser -- --nocapture`

Expected: PASS.

### Task 7: Add Trading Time And Strict Validation Helpers

**Files:**
- Modify: `src-tauri/src/services/quant.rs`

- [ ] **Step 1: Write tests for trading windows**

Add tests:

```rust
#[test]
fn trading_time_rejects_weekend() {
    let now = chrono::FixedOffset::east_opt(8 * 3600).unwrap()
        .with_ymd_and_hms(2026, 7, 11, 10, 0, 0).unwrap();
    assert!(!is_trading_time(now));
}

#[test]
fn trading_time_accepts_morning_session() {
    let now = chrono::FixedOffset::east_opt(8 * 3600).unwrap()
        .with_ymd_and_hms(2026, 7, 7, 10, 0, 0).unwrap();
    assert!(is_trading_time(now));
}

#[test]
fn strict_quote_rejects_outside_session_time() {
    let now = chrono::FixedOffset::east_opt(8 * 3600).unwrap()
        .with_ymd_and_hms(2026, 7, 7, 10, 0, 0).unwrap();
    let timestamp = chrono::FixedOffset::east_opt(8 * 3600).unwrap()
        .with_ymd_and_hms(2026, 7, 7, 8, 0, 0).unwrap()
        .timestamp();
    assert!(!quote_timestamp_is_current_session(timestamp, now));
}

#[test]
fn strict_quote_rejects_wrong_date_timestamp() {
    let now = chrono::FixedOffset::east_opt(8 * 3600).unwrap()
        .with_ymd_and_hms(2026, 7, 7, 10, 0, 0).unwrap();
    let timestamp = chrono::FixedOffset::east_opt(8 * 3600).unwrap()
        .with_ymd_and_hms(2026, 7, 6, 10, 0, 0).unwrap()
        .timestamp();
    assert!(!quote_timestamp_is_current_session(timestamp, now));
}

#[test]
fn strict_quote_rejects_stale_timestamp_even_with_open_status() {
    let quote = crate::services::quote::RealtimeStockQuote {
        price: 12.34,
        exchange_timestamp: Some(shanghai_time(2026, 7, 6, 10, 0, 0).timestamp()),
        market_status: Some("5".into()),
        quote_fetched_at: "2026-07-07 10:00:00".into(),
    };
    let now = shanghai_time(2026, 7, 7, 10, 0, 0);
    assert!(!quote_is_strictly_realtime(&quote, now));
}

#[test]
fn strict_quote_rejects_open_status_without_timestamp_outside_trading_time() {
    let quote = crate::services::quote::RealtimeStockQuote {
        price: 12.34,
        exchange_timestamp: None,
        market_status: Some("5".into()),
        quote_fetched_at: "2026-07-07 15:30:00".into(),
    };
    let after_close = shanghai_time(2026, 7, 7, 15, 30, 0);
    let weekend = shanghai_time(2026, 7, 11, 10, 0, 0);
    assert!(!quote_is_strictly_realtime(&quote, after_close));
    assert!(!quote_is_strictly_realtime(&quote, weekend));
}
```

- [ ] **Step 2: Implement helpers**

Add helpers that use `chrono::DateTime<chrono::FixedOffset>` with an explicit UTC+8 offset. Do not use the desktop local timezone for market decisions.

```rust
pub fn china_market_now() -> chrono::DateTime<chrono::FixedOffset> {
    let offset = chrono::FixedOffset::east_opt(8 * 3600).unwrap();
    chrono::Utc::now().with_timezone(&offset)
}

pub fn is_trading_time(now: chrono::DateTime<chrono::FixedOffset>) -> bool {
    use chrono::{Datelike, Timelike, Weekday};
    if matches!(now.weekday(), Weekday::Sat | Weekday::Sun) {
        return false;
    }
    let minutes = now.hour() * 60 + now.minute();
    let morning = (9 * 60 + 30..=11 * 60 + 30).contains(&minutes);
    let afternoon = (13 * 60..=15 * 60).contains(&minutes);
    morning || afternoon
}

pub fn next_refresh_at(now: chrono::DateTime<chrono::FixedOffset>, poll_interval_seconds: i64) -> Option<String> {
    if !is_trading_time(now) {
        return None;
    }
    Some((now + chrono::Duration::seconds(poll_interval_seconds)).format("%Y-%m-%d %H:%M:%S").to_string())
}

pub fn quote_timestamp_is_current_session(exchange_timestamp: i64, now: chrono::DateTime<chrono::FixedOffset>) -> bool {
    let quote_time = chrono::DateTime::from_timestamp(exchange_timestamp, 0)
        .map(|dt| dt.with_timezone(now.offset()));
    match quote_time {
        Some(ts) => ts.date_naive() == now.date_naive() && is_trading_time(ts),
        None => false,
    }
}

pub fn quote_status_allows_strict_signal(market_status: Option<&str>) -> bool {
    matches!(market_status.map(str::trim), Some("5") | Some("交易中"))
}

pub fn quote_is_strictly_realtime(quote: &crate::services::quote::RealtimeStockQuote, now: chrono::DateTime<chrono::FixedOffset>) -> bool {
    if let Some(ts) = quote.exchange_timestamp {
        return quote_timestamp_is_current_session(ts, now);
    }
    is_trading_time(now) && quote_status_allows_strict_signal(quote.market_status.as_deref())
}
```

`quote_is_strictly_realtime` must enforce: timestamp present and valid wins; timestamp present and invalid rejects even if status says open; only when timestamp is absent may open market status allow the quote.

Use Monday-Friday and `09:30-11:30`, `13:00-15:00`.

- [ ] **Step 3: Run quant tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml services::quant -- --nocapture`

Expected: PASS.

- [ ] **Step 4: Commit chunk 2**

Run only when commits are explicitly requested:

```bash
git add src-tauri/src/services/quant.rs src-tauri/src/services/mod.rs src-tauri/src/services/quote.rs
git commit -m "feat(quant): add signal engine"
```

## Chunk 3: Quant Commands And Dashboard Refresh

### Task 8: Implement Watchlist Commands

**Files:**
- Create: `src-tauri/src/commands/quant.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write failing database-helper tests**

In `src-tauri/src/commands/quant.rs`, add tests around helper functions that take `&rusqlite::Connection` instead of Tauri `State`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn test_conn() -> rusqlite::Connection {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        create_test_schema(&conn).unwrap();
        conn
    }

    fn create_test_schema(conn: &rusqlite::Connection) -> Result<(), rusqlite::Error> {
        conn.execute_batch(r#"
            CREATE TABLE accounts (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, type TEXT NOT NULL DEFAULT 'cash', balance REAL NOT NULL DEFAULT 0, created_at TEXT NOT NULL DEFAULT '2026-07-07 09:00:00');
            CREATE TABLE categories (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, type TEXT NOT NULL, icon TEXT DEFAULT 'pushpin');
            CREATE TABLE transactions (id INTEGER PRIMARY KEY AUTOINCREMENT, type TEXT NOT NULL, amount REAL NOT NULL, category_id INTEGER, account_id INTEGER, transfer_to_account_id INTEGER, note TEXT DEFAULT '', transaction_date TEXT NOT NULL, created_at TEXT NOT NULL DEFAULT '2026-07-07 09:00:00');
            CREATE TABLE holdings (id INTEGER PRIMARY KEY AUTOINCREMENT, code TEXT NOT NULL, name TEXT NOT NULL, type TEXT NOT NULL DEFAULT 'stock', quantity REAL NOT NULL, cost_price REAL NOT NULL, current_price REAL NOT NULL DEFAULT 0, market TEXT NOT NULL DEFAULT 'cn', created_at TEXT NOT NULL DEFAULT '2026-07-07 09:00:00', updated_at TEXT NOT NULL DEFAULT '2026-07-07 09:00:00');
            CREATE TABLE price_history (id INTEGER PRIMARY KEY AUTOINCREMENT, holding_id INTEGER NOT NULL, price REAL NOT NULL, recorded_at TEXT NOT NULL);
            CREATE TABLE quant_watchlist (id INTEGER PRIMARY KEY AUTOINCREMENT, code TEXT NOT NULL, name TEXT NOT NULL, market TEXT NOT NULL DEFAULT 'cn', enabled INTEGER NOT NULL DEFAULT 1, created_at TEXT NOT NULL DEFAULT '2026-07-07 09:00:00', updated_at TEXT NOT NULL DEFAULT '2026-07-07 09:00:00', UNIQUE(code, market));
            CREATE TABLE quant_strategy_settings (id INTEGER PRIMARY KEY AUTOINCREMENT, code TEXT NOT NULL, market TEXT NOT NULL DEFAULT 'cn', ma_short INTEGER NOT NULL DEFAULT 5, ma_long INTEGER NOT NULL DEFAULT 20, grid_lookback_days INTEGER NOT NULL DEFAULT 20, poll_interval_seconds INTEGER NOT NULL DEFAULT 60, desktop_notification_enabled INTEGER NOT NULL DEFAULT 1, enabled INTEGER NOT NULL DEFAULT 1, created_at TEXT NOT NULL DEFAULT '2026-07-07 09:00:00', updated_at TEXT NOT NULL DEFAULT '2026-07-07 09:00:00', UNIQUE(code, market));
            CREATE TABLE quant_signals (id INTEGER PRIMARY KEY AUTOINCREMENT, code TEXT NOT NULL, name TEXT NOT NULL, market TEXT NOT NULL DEFAULT 'cn', direction TEXT NOT NULL, trigger_price REAL NOT NULL, trend_state TEXT NOT NULL, source TEXT NOT NULL, trigger_zone TEXT NOT NULL, dedupe_key TEXT NOT NULL, triggered_at TEXT NOT NULL, created_at TEXT NOT NULL DEFAULT '2026-07-07 09:00:00');
        "#)
    }

    #[test]
    fn add_watchlist_rejects_duplicate_code_market() {
        let conn = test_conn();
        add_watchlist_to_conn(&conn, NewQuantWatchlistItem { code: "600000".into(), name: Some("浦发银行".into()), market: Some("cn".into()), enabled: Some(true) }).unwrap();

        let err = add_watchlist_to_conn(&conn, NewQuantWatchlistItem { code: "600000".into(), name: Some("重复".into()), market: Some("cn".into()), enabled: Some(true) }).unwrap_err();

        assert!(err.contains("已存在") || err.contains("重复"));
    }

    #[test]
    fn add_watchlist_rejects_empty_code() {
        let conn = test_conn();

        let err = add_watchlist_to_conn(&conn, NewQuantWatchlistItem { code: " ".into(), name: None, market: Some("cn".into()), enabled: Some(true) }).unwrap_err();

        assert!(err.contains("代码"));
    }

    #[test]
    fn update_watchlist_changes_name_and_initial_enabled() {
        let conn = test_conn();
        let item = add_watchlist_to_conn(&conn, NewQuantWatchlistItem { code: "600000".into(), name: Some("旧名".into()), market: Some("cn".into()), enabled: Some(true) }).unwrap();

        let updated = update_watchlist_in_conn(&conn, item.id, QuantWatchlistItemUpdate { name: "浦发银行".into(), enabled: false }).unwrap();

        assert_eq!(updated.name, "浦发银行");
        assert!(!updated.enabled);
    }
}
```

`test_conn()` should create an in-memory DB and execute the same quant table DDL used by `db.rs`.

- [ ] **Step 2: Run tests to verify failure**

Run: `cargo test --manifest-path src-tauri/Cargo.toml commands::quant::tests -- --nocapture`

Expected: FAIL because helpers are not implemented.

- [ ] **Step 3: Create command file with watchlist CRUD**

Implement:

```rust
#[tauri::command]
pub fn list_quant_watchlist(state: State<AppState>) -> Result<Vec<QuantWatchlistItem>, String> { ... }

#[tauri::command]
pub async fn add_quant_watchlist(state: State<'_, AppState>, input: NewQuantWatchlistItem) -> Result<QuantWatchlistItem, String> { ... }

#[tauri::command]
pub fn update_quant_watchlist(state: State<AppState>, id: i64, input: QuantWatchlistItemUpdate) -> Result<QuantWatchlistItem, String> { ... }

#[tauri::command]
pub fn delete_quant_watchlist(state: State<AppState>, id: i64) -> Result<(), String> { ... }
```

Use `lookup_stock_name` when `input.name` is empty. Reject empty code. Return a friendly duplicate error for `UNIQUE(code, market)` violations.

- [ ] **Step 4: Re-export and register commands**

In `commands/mod.rs`, add `pub mod quant;` and re-export functions. In `lib.rs`, register commands in `generate_handler!`.

- [ ] **Step 5: Run tests and compile**

Run: `cargo test --manifest-path src-tauri/Cargo.toml commands::quant::tests -- --nocapture`

Expected: PASS.

Run: `cargo check --manifest-path src-tauri/Cargo.toml`

Expected: PASS or failures only for upcoming unimplemented command references if all command names were added at once.

### Task 9: Implement Target Merge And Settings Update

**Files:**
- Modify: `src-tauri/src/commands/quant.rs`

- [ ] **Step 1: Write failing target/settings tests**

Add tests:

```rust
#[test]
fn merge_targets_collapses_duplicate_holdings_and_watchlist() {
    let conn = test_conn();
    conn.execute("INSERT INTO holdings (code, name, type, quantity, cost_price, current_price, market) VALUES ('600000', '旧名', 'stock', 100, 10, 10, 'cn')", []).unwrap();
    conn.execute("INSERT INTO holdings (code, name, type, quantity, cost_price, current_price, market) VALUES ('600000', '浦发银行', 'stock', 200, 11, 11, 'cn')", []).unwrap();
    conn.execute("INSERT INTO quant_watchlist (code, name, market, enabled) VALUES ('600000', 'Watch Name', 'cn', 1)", []).unwrap();

    let targets = merge_quant_targets(&conn).unwrap();

    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].source, "holding_watchlist");
    assert_eq!(targets[0].name, "浦发银行");
}

#[test]
fn watchlist_disabled_default_stays_disabled_after_settings_insert() {
    let conn = test_conn();
    conn.execute("INSERT INTO quant_watchlist (code, name, market, enabled) VALUES ('600000', '浦发银行', 'cn', 0)", []).unwrap();

    ensure_quant_settings(&conn, "600000", "cn", "watchlist", Some(false)).unwrap();

    let enabled: i64 = conn.query_row("SELECT enabled FROM quant_strategy_settings WHERE code = '600000'", [], |row| row.get(0)).unwrap();
    assert_eq!(enabled, 0);
}

#[test]
fn settings_enabled_overrides_watchlist_initial_enabled() {
    let conn = test_conn();
    conn.execute("INSERT INTO quant_watchlist (code, name, market, enabled) VALUES ('600000', '浦发银行', 'cn', 0)", []).unwrap();
    conn.execute("INSERT INTO quant_strategy_settings (code, market, enabled) VALUES ('600000', 'cn', 1)", []).unwrap();

    let targets = merge_quant_targets(&conn).unwrap();

    assert!(targets[0].enabled);
}

#[test]
fn desktop_notification_setting_does_not_disable_signal_generation() {
    let conn = test_conn();
    conn.execute("INSERT INTO holdings (code, name, type, quantity, cost_price, current_price, market) VALUES ('600000', '浦发银行', 'stock', 100, 10, 10, 'cn')", []).unwrap();
    conn.execute("INSERT INTO quant_strategy_settings (code, market, enabled, desktop_notification_enabled) VALUES ('600000', 'cn', 1, 0)", []).unwrap();

    let targets = merge_quant_targets(&conn).unwrap();

    assert!(targets[0].enabled);
    assert!(!targets[0].desktop_notification_enabled);
}

#[test]
fn deleting_watchlist_keeps_held_target_available() {
    let conn = test_conn();
    conn.execute("INSERT INTO holdings (code, name, type, quantity, cost_price, current_price, market) VALUES ('600000', '浦发银行', 'stock', 100, 10, 10, 'cn')", []).unwrap();
    let id = insert_watchlist(&conn, "600000", "cn", true).unwrap();

    delete_watchlist_from_conn(&conn, id).unwrap();
    let targets = merge_quant_targets(&conn).unwrap();

    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].source, "holding");
}
```

- [ ] **Step 2: Run tests to verify failure**

Run: `cargo test --manifest-path src-tauri/Cargo.toml commands::quant::tests -- --nocapture`

Expected: FAIL because target merge/settings helpers are not implemented.

- [ ] **Step 3: Add settings helper**

Implement `ensure_quant_settings(conn, code, market, source, watchlist_enabled)` so lazy insertion copies watchlist enabled for watchlist-only targets and defaults holding targets to enabled.

- [ ] **Step 4: Implement target list**

Implement `list_quant_targets`:

```rust
#[tauri::command]
pub fn list_quant_targets(state: State<AppState>) -> Result<Vec<QuantTarget>, String> { ... }
```

Collapse holdings and watchlist by `(code, market)`. Do not include funds. If both sources exist, return `source = "holding_watchlist"`. If multiple holding rows have the same `(code, market)`, use the latest non-empty holding name by highest `id`; fall back to watchlist name, then code. Deleting a watchlist row must remove only the watchlist source and must not hide a held stock target.

- [ ] **Step 5: Implement settings update command**

Implement:

```rust
#[tauri::command]
pub fn update_quant_strategy_settings(
    state: State<AppState>,
    code: String,
    market: String,
    input: QuantStrategySettingsUpdate,
) -> Result<QuantTarget, String> { ... }
```

This command upserts settings and returns the merged target.

- [ ] **Step 6: Run tests and compile**

Run: `cargo test --manifest-path src-tauri/Cargo.toml commands::quant::tests -- --nocapture`

Expected: PASS.

Run: `cargo check --manifest-path src-tauri/Cargo.toml`

Expected: PASS.

### Task 10: Implement Signal History Filtering

**Files:**
- Modify: `src-tauri/src/commands/quant.rs`

- [ ] **Step 1: Write failing signal history tests**

Add tests:

```rust
#[test]
fn list_signals_filters_by_code_direction_and_limit() {
    let conn = test_conn();
    insert_signal(&conn, "600000", "buy_attention", "2026-07-07 10:00:00").unwrap();
    insert_signal(&conn, "000001", "sell_attention", "2026-07-07 10:01:00").unwrap();

    let rows = list_quant_signals_from_conn(&conn, Some(QuantSignalFilter {
        code: Some("600000".into()),
        direction: Some("buy_attention".into()),
        limit: Some(10),
        ..Default::default()
    })).unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].code, "600000");
    assert_eq!(rows[0].direction, "buy_attention");
}

#[test]
fn list_signals_defaults_to_recent_100() {
    let conn = test_conn();
    for i in 0..105 {
        insert_signal(&conn, "600000", "buy_attention", &format!("2026-07-07 10:{:02}:00", i % 60)).unwrap();
    }

    let rows = list_quant_signals_from_conn(&conn, None).unwrap();

    assert_eq!(rows.len(), 100);
}
```

- [ ] **Step 2: Implement signal history query**

Implement:

```rust
#[tauri::command]
pub fn list_quant_signals(state: State<AppState>, filter: Option<QuantSignalFilter>) -> Result<Vec<QuantSignal>, String> { ... }
```

Default `limit` to 100. Support optional `code`, `direction`, `from`, and `to`.

- [ ] **Step 3: Run tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml commands::quant::tests -- --nocapture`

Expected: PASS.

### Task 11: Implement Dashboard Construction Without Persistence

**Files:**
- Modify: `src-tauri/src/commands/quant.rs`

- [ ] **Step 1: Define snapshot DTO used by dashboard and refresh helpers**

Before dashboard tests, define the shared in-memory snapshot shape in `quant.rs`:

```rust
struct QuantMarketSnapshot {
    code: String,
    market: String,
    daily_bars: Result<Vec<KlineBar>, String>,
    realtime_quote: Result<RealtimeStockQuote, String>,
}
```

Test fixtures must construct this directly with helpers such as `buy_signal_snapshot`, `watch_snapshot`, and `quote_error_snapshot`. These helpers should return daily bars plus realtime quote data and must not hit the network.

- [ ] **Step 2: Write failing dashboard tests**

Add tests for a helper such as `build_quant_dashboard_with_snapshots`:

```rust
#[test]
fn dashboard_does_not_persist_signals() {
    let conn = test_conn_with_enabled_holding("600000");
    let snapshots = vec![buy_signal_snapshot("600000", "cn")];
    let now = shanghai_time(2026, 7, 7, 10, 0, 0);

    let dashboard = build_quant_dashboard_with_snapshots(&conn, &snapshots, now).unwrap();

    assert_eq!(dashboard.targets[0].output_state, "buy_attention");
    assert!(dashboard.targets[0].quote_fetched_at.is_some());
    assert_eq!(count_rows(&conn, "quant_signals"), 0);
}

#[test]
fn per_target_quote_error_does_not_hide_other_targets() {
    let conn = test_conn();
    insert_enabled_holding(&conn, "600000").unwrap();
    insert_enabled_holding(&conn, "000001").unwrap();
    let snapshots = vec![quote_error_snapshot("600000", "cn", "缺少交易时间"), watch_snapshot("000001", "cn")];
    let now = shanghai_time(2026, 7, 7, 10, 0, 0);

    let dashboard = build_quant_dashboard_with_snapshots(&conn, &snapshots, now).unwrap();

    assert_eq!(dashboard.targets.len(), 2);
    assert!(dashboard.targets.iter().any(|t| t.code == "600000" && t.output_state == "quote_error"));
    assert!(dashboard.targets.iter().any(|t| t.code == "000001" && t.output_state == "watch"));
}
```

- [ ] **Step 3: Add `get_quant_dashboard`**

Implement dashboard without creating new signal records. It must remain backend-owned: merge targets, load settings, load live snapshots for enabled targets, compute completed daily-bar trend/grid from snapshots, validate strict realtime quote from snapshots, and set each target's `output_state`, `current_trigger_zone`, `quote_fetched_at`, and `last_error`. It must not persist `quant_signals` and must not ask the frontend to calculate trend/grid.

It must also populate:

- `QuantDashboard.recent_signals` with `list_quant_signals_from_conn(conn, Some(QuantSignalFilter { limit: Some(100), ..Default::default() }))`.
- each `QuantTarget.latest_signal` with that target's most recent persisted signal by `(code, market)`.

Use a thin async Tauri wrapper:

```rust
#[tauri::command]
pub async fn get_quant_dashboard(state: State<'_, AppState>) -> Result<QuantDashboard, String> {
    let now = crate::services::quant::china_market_now();
    let snapshots = load_live_quant_snapshots(&state, now).await?;
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    build_quant_dashboard_with_snapshots(&conn, &snapshots, now)
}
```

- [ ] **Step 4: Run dashboard tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml commands::quant::tests -- --nocapture`

Expected: PASS.

### Task 12: Implement Refresh, Cooldown, Persistence, And Generated Signals

**Files:**
- Modify: `src-tauri/src/commands/quant.rs`

- [ ] **Step 1: Write failing refresh tests**

Use the `QuantMarketSnapshot` type from Task 11. Use a synchronous evaluator for tests so refresh behavior is deterministic and does not require async test dependencies. The async Tauri command should fetch live market data, convert it into in-memory `QuantMarketSnapshot` values, then call this same synchronous evaluator.

```rust
fn refresh_quant_signals_with_snapshots(
    conn: &rusqlite::Connection,
    snapshots: &[QuantMarketSnapshot],
    now: chrono::DateTime<chrono::FixedOffset>,
) -> Result<QuantRefreshResult, String> { ... }

async fn refresh_quant_signals_with_snapshot_loader<L: QuantSnapshotLoader>(
    state: &AppState,
    loader: &L,
    now: chrono::DateTime<chrono::FixedOffset>,
) -> Result<QuantRefreshResult, String> { ... }
```

`QuantSnapshotLoader` should expose only read-only K-line and realtime quote loading. It must not have any method that updates holdings or price history. The mock loader's `refresh_quotes_call_count()` is a guard that should remain `0`; if the implementation cannot provide that exact method, use an equivalent assertion proving the existing mutating `refresh_quotes` path was not called.

Do not hit live network in tests.

Required tests:

```rust
#[test]
fn market_closed_refresh_persists_no_signals() {
    let conn = test_conn_with_enabled_holding("600000");
    let snapshots = vec![buy_signal_snapshot("600000", "cn")];
    let now = shanghai_time(2026, 7, 7, 15, 30, 0);

    let result = refresh_quant_signals_with_snapshots(&conn, &snapshots, now).unwrap();

    assert!(result.generated_signals.is_empty());
    assert_eq!(count_rows(&conn, "quant_signals"), 0);
}

#[test]
fn refresh_persists_generated_signal_after_cooldown() {
    let conn = test_conn_with_enabled_holding("600000");
    let snapshots = vec![buy_signal_snapshot("600000", "cn")];
    let now = shanghai_time(2026, 7, 7, 10, 0, 0);

    let result = refresh_quant_signals_with_snapshots(&conn, &snapshots, now).unwrap();

    assert_eq!(result.generated_signals.len(), 1);
    assert_eq!(result.generated_signals[0].signal.direction, "buy_attention");
    assert_eq!(count_rows(&conn, "quant_signals"), 1);
}

#[test]
fn refresh_suppresses_duplicate_within_five_minutes() {
    let conn = test_conn_with_enabled_holding("600000");
    insert_signal_with_dedupe(&conn, "cn:600000:buy_attention:buy_1", "2026-07-07 10:00:00").unwrap();
    let snapshots = vec![buy_signal_snapshot("600000", "cn")];
    let now = shanghai_time(2026, 7, 7, 10, 4, 0);

    let result = refresh_quant_signals_with_snapshots(&conn, &snapshots, now).unwrap();

    assert!(result.generated_signals.is_empty());
    assert_eq!(count_rows(&conn, "quant_signals"), 1);
}

#[test]
fn refresh_allows_same_identity_after_five_minutes() {
    let conn = test_conn_with_enabled_holding("600000");
    insert_signal_with_dedupe(&conn, "cn:600000:buy_attention:buy_1", "2026-07-07 10:00:00").unwrap();
    let snapshots = vec![buy_signal_snapshot("600000", "cn")];
    let now = shanghai_time(2026, 7, 7, 10, 5, 1);

    let result = refresh_quant_signals_with_snapshots(&conn, &snapshots, now).unwrap();

    assert_eq!(result.generated_signals.len(), 1);
    assert_eq!(count_rows(&conn, "quant_signals"), 2);
}

#[test]
fn cooldown_identity_differs_by_market_direction_and_zone() {
    let conn = test_conn();
    insert_enabled_holding_with_market(&conn, "600000", "cn").unwrap();
    insert_enabled_holding_with_market(&conn, "600000", "hk").unwrap();
    insert_signal_with_dedupe(&conn, "cn:600000:buy_attention:buy_1", "2026-07-07 10:00:00").unwrap();
    let snapshots = vec![
        buy_signal_snapshot("600000", "hk"),
        sell_signal_snapshot("600000", "cn"),
        buy_signal_zone_snapshot("600000", "cn", "buy_2"),
    ];
    let now = shanghai_time(2026, 7, 7, 10, 1, 0);

    let result = refresh_quant_signals_with_snapshots(&conn, &snapshots, now).unwrap();

    assert_eq!(result.generated_signals.len(), 3);
}

#[test]
fn generated_signal_carries_desktop_notification_enabled_flag() {
    let conn = test_conn_with_enabled_holding("600000");
    conn.execute("UPDATE quant_strategy_settings SET desktop_notification_enabled = 0 WHERE code = '600000'", []).unwrap();
    let snapshots = vec![buy_signal_snapshot("600000", "cn")];
    let now = shanghai_time(2026, 7, 7, 10, 0, 0);

    let result = refresh_quant_signals_with_snapshots(&conn, &snapshots, now).unwrap();

    assert_eq!(result.generated_signals.len(), 1);
    assert!(!result.generated_signals[0].desktop_notification_enabled);
}

#[test]
fn quote_error_persists_no_signal_and_does_not_stop_other_targets() {
    let conn = test_conn();
    insert_enabled_holding(&conn, "600000").unwrap();
    insert_enabled_holding(&conn, "000001").unwrap();
    let snapshots = vec![quote_error_snapshot("600000", "cn", "缺少交易时间"), buy_signal_snapshot("000001", "cn")];
    let now = shanghai_time(2026, 7, 7, 10, 0, 0);

    let result = refresh_quant_signals_with_snapshots(&conn, &snapshots, now).unwrap();

    assert_eq!(result.generated_signals.len(), 1);
    assert_eq!(result.generated_signals[0].signal.code, "000001");
    assert_eq!(count_rows(&conn, "quant_signals"), 1);
}

#[test]
fn strict_realtime_missing_timestamp_or_status_generates_no_signal() {
    let conn = test_conn_with_enabled_holding("600000");
    let snapshots = vec![missing_timestamp_and_status_snapshot("600000", "cn")];
    let now = shanghai_time(2026, 7, 7, 10, 0, 0);

    let result = refresh_quant_signals_with_snapshots(&conn, &snapshots, now).unwrap();

    assert!(result.generated_signals.is_empty());
    assert_eq!(result.dashboard.targets[0].output_state, "quote_error");
    assert_eq!(count_rows(&conn, "quant_signals"), 0);
}

#[test]
fn strict_realtime_stale_or_wrong_session_timestamp_generates_no_signal() {
    let conn = test_conn_with_enabled_holding("600000");
    let stale = shanghai_time(2026, 7, 6, 10, 0, 0).timestamp();
    let snapshots = vec![timestamp_snapshot("600000", "cn", stale)];
    let now = shanghai_time(2026, 7, 7, 10, 0, 0);

    let result = refresh_quant_signals_with_snapshots(&conn, &snapshots, now).unwrap();

    assert!(result.generated_signals.is_empty());
    assert_eq!(result.dashboard.targets[0].output_state, "quote_error");
    assert_eq!(count_rows(&conn, "quant_signals"), 0);
}

#[test]
fn strict_realtime_stale_timestamp_with_open_status_still_generates_no_signal() {
    let conn = test_conn_with_enabled_holding("600000");
    let stale = shanghai_time(2026, 7, 6, 10, 0, 0).timestamp();
    let snapshots = vec![timestamp_and_status_snapshot("600000", "cn", stale, "5")];
    let now = shanghai_time(2026, 7, 7, 10, 0, 0);

    let result = refresh_quant_signals_with_snapshots(&conn, &snapshots, now).unwrap();

    assert!(result.generated_signals.is_empty());
    assert_eq!(result.dashboard.targets[0].output_state, "quote_error");
    assert_eq!(count_rows(&conn, "quant_signals"), 0);
}

#[test]
fn strict_realtime_closed_status_generates_no_signal() {
    let conn = test_conn_with_enabled_holding("600000");
    let snapshots = vec![market_status_snapshot("600000", "cn", "0")];
    let now = shanghai_time(2026, 7, 7, 10, 0, 0);

    let result = refresh_quant_signals_with_snapshots(&conn, &snapshots, now).unwrap();

    assert!(result.generated_signals.is_empty());
    assert_eq!(result.dashboard.targets[0].output_state, "quote_error");
    assert_eq!(count_rows(&conn, "quant_signals"), 0);
}

#[test]
fn strict_realtime_current_session_timestamp_allows_signal() {
    let conn = test_conn_with_enabled_holding("600000");
    let timestamp = shanghai_time(2026, 7, 7, 10, 0, 0).timestamp();
    let snapshots = vec![timestamp_buy_signal_snapshot("600000", "cn", timestamp)];
    let now = shanghai_time(2026, 7, 7, 10, 0, 0);

    let result = refresh_quant_signals_with_snapshots(&conn, &snapshots, now).unwrap();

    assert_eq!(result.generated_signals.len(), 1);
}

#[test]
fn strict_realtime_open_market_status_without_timestamp_allows_signal() {
    let conn = test_conn_with_enabled_holding("600000");
    let snapshots = vec![market_status_buy_signal_snapshot("600000", "cn", "5")];
    let now = shanghai_time(2026, 7, 7, 10, 0, 0);

    let result = refresh_quant_signals_with_snapshots(&conn, &snapshots, now).unwrap();

    assert_eq!(result.generated_signals.len(), 1);
}

#[test]
fn disabled_targets_are_not_polled_or_persisted() {
    let conn = test_conn_with_enabled_holding("600000");
    conn.execute("UPDATE quant_strategy_settings SET enabled = 0 WHERE code = '600000'", []).unwrap();
    let snapshots = vec![buy_signal_snapshot("600000", "cn")];
    let now = shanghai_time(2026, 7, 7, 10, 0, 0);

    let result = refresh_quant_signals_with_snapshots(&conn, &snapshots, now).unwrap();

    assert!(result.generated_signals.is_empty());
    assert_eq!(result.dashboard.targets[0].enabled, false);
    assert_eq!(count_rows(&conn, "quant_signals"), 0);
}

#[test]
fn refresh_does_not_mutate_finance_tables() {
    let conn = test_conn_with_enabled_holding("600000");
    conn.execute("INSERT INTO accounts (id, name, type, balance) VALUES (99, '证券账户', 'broker', 1000)", []).unwrap();
    conn.execute("INSERT INTO price_history (holding_id, price, recorded_at) VALUES (1, 10.0, '2026-07-07 09:30:00')", []).unwrap();
    let before = finance_table_snapshot(&conn);
    let snapshots = vec![buy_signal_snapshot("600000", "cn")];
    let now = shanghai_time(2026, 7, 7, 10, 0, 0);

    refresh_quant_signals_with_snapshots(&conn, &snapshots, now).unwrap();

    assert_eq!(finance_table_snapshot(&conn), before);
}

#[tokio::test]
async fn command_path_refresh_does_not_mutate_finance_tables() {
    let state = test_app_state_with_enabled_holding("600000");
    let before = finance_table_snapshot_from_state(&state);
    let loader = MockSnapshotLoader::with_snapshots(vec![buy_signal_snapshot("600000", "cn")]);
    let now = shanghai_time(2026, 7, 7, 10, 0, 0);

    refresh_quant_signals_with_snapshot_loader(&state, &loader, now).await.unwrap();

    assert_eq!(finance_table_snapshot_from_state(&state), before);
    assert_eq!(loader.refresh_quotes_call_count(), 0);
}
```

`refresh_does_not_mutate_finance_tables` must snapshot counts and representative values from `holdings`, `accounts`, `transactions`, and `price_history` before and after refresh.

- [ ] **Step 2: Run tests to verify failure**

Run: `cargo test --manifest-path src-tauri/Cargo.toml commands::quant::tests -- --nocapture`

Expected: FAIL because refresh helper is not implemented.

- [ ] **Step 3: Add `refresh_quant_signals` helper and command**

Implement:

```rust
#[tauri::command]
pub async fn refresh_quant_signals(state: State<'_, AppState>) -> Result<QuantRefreshResult, String> { ... }
```

For each enabled target:

- Fetch daily bars with existing `fetch_stock_kline(code, "day", limit)` where `limit = max(ma_long, grid_lookback_days) + 1`; with default MA20/grid20 this is 21 bars.
- Exclude unfinished current-day bar before 15:00.
- Compute trend/grid in backend.
- Fetch strict realtime quote with `fetch_stock_realtime_quote(code, market)`.
- Validate strict realtime quote: accept either a current-session exchange timestamp or explicit open-market status; otherwise set `quote_error` and persist no signal for that target.
- Decide current output state.
- If output is buy/sell, build dedupe key `market:code:direction:trigger_zone`.
- Check last `quant_signals` row for that dedupe key in the last 5 minutes.
- Insert a new row only when cooldown allows.
- Return newly inserted signals as `QuantGeneratedSignal` with the target's `desktop_notification_enabled`.
- Return `dashboard.recent_signals` including the newly inserted rows and set each target's `latest_signal` from persisted history after insertions.

When `is_trading_time(now)` is false, return a dashboard but do not persist signals and return `generated_signals: []`.

Keep the Tauri command wrapper thin:

```rust
#[tauri::command]
pub async fn refresh_quant_signals(state: State<'_, AppState>) -> Result<QuantRefreshResult, String> {
    let now = crate::services::quant::china_market_now();
    let snapshots = load_live_quant_snapshots(&state, now).await?;
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    refresh_quant_signals_with_snapshots(&conn, &snapshots, now)
}
```

`load_live_quant_snapshots` must not fail the whole refresh when one target's K-line or realtime quote fetch fails. It should return one `QuantMarketSnapshot` per enabled target, storing per-target failures in `daily_bars: Err(...)` or `realtime_quote: Err(...)`. Only unrecoverable application errors such as database lock failure should return `Err` from the command.

Define the loader boundary explicitly:

```rust
trait QuantSnapshotLoader {
    async fn load_snapshot(&self, target: &QuantTarget, settings: &QuantStrategySettingsRow) -> QuantMarketSnapshot;
}

struct LiveQuantSnapshotLoader;

impl QuantSnapshotLoader for LiveQuantSnapshotLoader {
    async fn load_snapshot(&self, target: &QuantTarget, settings: &QuantStrategySettingsRow) -> QuantMarketSnapshot {
        let limit = settings.ma_long.max(settings.grid_lookback_days) + 1;
        QuantMarketSnapshot {
            code: target.code.clone(),
            market: target.market.clone(),
            daily_bars: crate::services::quote::fetch_stock_kline(&target.code, "day", limit).await,
            realtime_quote: crate::services::quote::fetch_stock_realtime_quote(&target.code, &target.market).await,
        }
    }
}
```

Target selection flow:

- Lock DB only long enough to merge targets and load settings.
- Drop the DB lock before awaiting K-line or realtime quote network calls.
- Load snapshots for enabled targets only.
- Re-lock DB after network calls to evaluate and persist signals.
- Never call `commands::refresh_quotes` or `commands::holdings::refresh_all_quotes` from quant code.

- [ ] **Step 4: Run refresh tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml commands::quant::tests -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Register all quant commands**

Ensure `lib.rs` registers:

```rust
commands::list_quant_watchlist,
commands::add_quant_watchlist,
commands::update_quant_watchlist,
commands::delete_quant_watchlist,
commands::update_quant_strategy_settings,
commands::list_quant_targets,
commands::get_quant_dashboard,
commands::refresh_quant_signals,
commands::list_quant_signals,
```

- [ ] **Step 6: Run backend checks**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -- --nocapture`

Expected: PASS.

- [ ] **Step 7: Commit chunk 3**

Run only when commits are explicitly requested:

```bash
git add src-tauri/src/commands/quant.rs src-tauri/src/commands/mod.rs src-tauri/src/lib.rs
git commit -m "feat(quant): add alert commands"
```

## Chunk 4: Desktop Notification Plugin

### Task 13: Add Tauri Notification Dependencies And Permissions

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/capabilities/default.json`
- Modify: `package.json`
- Modify: `package-lock.json`

- [ ] **Step 1: Install frontend plugin**

Run: `npm install @tauri-apps/plugin-notification`

Expected: `package.json` and `package-lock.json` update.

- [ ] **Step 2: Add Rust plugin**

Add to `src-tauri/Cargo.toml`:

```toml
tauri-plugin-notification = "2"
```

- [ ] **Step 3: Initialize plugin**

In `src-tauri/src/lib.rs`, add before global shortcut plugin or after opener plugin:

```rust
.plugin(tauri_plugin_notification::init())
```

- [ ] **Step 4: Add capability permission**

In `src-tauri/capabilities/default.json`, add:

```json
"notification:default"
```

If build reports finer-grained generated permissions are required, add the exact allow permissions named by Tauri's error message.

- [ ] **Step 5: Run build checks**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`

Expected: PASS.

Run: `npm run typecheck`

Expected: PASS after frontend notification use is added in Chunk 5.

## Chunk 5: Frontend Types, API, And Page

### Task 14: Add Quant Types And API Wrappers

**Files:**
- Modify: `src/types/index.ts`
- Modify: `src/api/index.ts`

- [ ] **Step 1: Add TypeScript types**

Add interfaces matching the spec DTOs:

```ts
export type QuantDirection = "buy_attention" | "sell_attention";
export type QuantTrendState = "bullish" | "bearish" | "neutral" | "insufficient_data";
export type QuantOutputState = QuantDirection | "watch" | "quote_error";
export type QuantTriggerZone = "buy_1" | "buy_2" | "sell_1" | "sell_2";

export interface QuantWatchlistItem {
  id: number;
  code: string;
  name: string;
  market: string;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface NewQuantWatchlistItem {
  code: string;
  name?: string;
  market?: string;
  enabled?: boolean;
}

export interface QuantWatchlistItemUpdate {
  name: string;
  enabled: boolean;
}

export interface QuantStrategySettingsUpdate {
  enabled: boolean;
  desktop_notification_enabled: boolean;
}

export interface QuantGridZone {
  zone: QuantTriggerZone;
  direction: QuantDirection;
  lower: number;
  upper: number;
}

export interface QuantSignal {
  id: number;
  code: string;
  name: string;
  market: string;
  direction: QuantDirection;
  trigger_price: number;
  trend_state: "bullish" | "bearish" | "neutral";
  source: "auto_grid";
  trigger_zone: QuantTriggerZone;
  triggered_at: string;
}

export interface QuantGeneratedSignal { signal: QuantSignal; desktop_notification_enabled: boolean; }

export interface QuantTarget {
  code: string;
  name: string;
  market: string;
  source: "holding" | "watchlist" | "holding_watchlist";
  enabled: boolean;
  desktop_notification_enabled: boolean;
  current_price: number | null;
  quote_fetched_at: string | null;
  trend_state: QuantTrendState;
  output_state: QuantOutputState;
  current_trigger_zone: QuantTriggerZone | null;
  ma_short: number | null;
  ma_long: number | null;
  grid_zones: QuantGridZone[];
  latest_signal: QuantSignal | null;
  last_error: string | null;
}

export interface QuantDashboard {
  is_trading_time: boolean;
  next_refresh_at: string | null;
  targets: QuantTarget[];
  recent_signals: QuantSignal[];
}

export interface QuantRefreshResult {
  dashboard: QuantDashboard;
  generated_signals: QuantGeneratedSignal[];
}

export interface QuantSignalFilter {
  code?: string;
  direction?: QuantDirection;
  from?: string;
  to?: string;
  limit?: number;
}
```

- [ ] **Step 2: Add API wrappers**

In `src/api/index.ts`, import quant types and add methods:

```ts
listQuantWatchlist: () => invoke<QuantWatchlistItem[]>("list_quant_watchlist"),
addQuantWatchlist: (input: NewQuantWatchlistItem) => invoke<QuantWatchlistItem>("add_quant_watchlist", { input }),
updateQuantWatchlist: (id: number, input: QuantWatchlistItemUpdate) => invoke<QuantWatchlistItem>("update_quant_watchlist", { id, input }),
deleteQuantWatchlist: (id: number) => invoke<void>("delete_quant_watchlist", { id }),
updateQuantStrategySettings: (code: string, market: string, input: QuantStrategySettingsUpdate) =>
  invoke<QuantTarget>("update_quant_strategy_settings", { code, market, input }),
listQuantTargets: () => invoke<QuantTarget[]>("list_quant_targets"),
getQuantDashboard: () => invoke<QuantDashboard>("get_quant_dashboard"),
refreshQuantSignals: () => invoke<QuantRefreshResult>("refresh_quant_signals"),
listQuantSignals: (filter?: QuantSignalFilter) => invoke<QuantSignal[]>("list_quant_signals", { filter }),
```

- [ ] **Step 3: Run typecheck**

Run: `npm run typecheck`

Expected: PASS until page route is added, or compile failures only from upcoming page imports if route was added first.

### Task 15: Add Notification Helper And Build Quant Alerts Page

**Files:**
- Create: `src/utils/quantNotifications.ts`
- Create: `src/pages/QuantAlerts.tsx`
- Modify: `src/styles/global.css`

- [ ] **Step 1: Create notification helper**

Create `src/utils/quantNotifications.ts`:

```ts
import { isPermissionGranted, requestPermission, sendNotification } from "@tauri-apps/plugin-notification";
import type { QuantGeneratedSignal } from "../types";

export async function notifyGeneratedQuantSignals(generatedSignals: QuantGeneratedSignal[]) {
  const eligible = generatedSignals.filter((item) => item.desktop_notification_enabled);
  if (eligible.length === 0) return { sent: 0, denied: false };

  let granted = await isPermissionGranted();
  if (!granted) {
    const permission = await requestPermission();
    granted = permission === "granted";
  }
  if (!granted) return { sent: 0, denied: true };

  for (const item of eligible) {
    sendNotification({
      title: `量化提醒：${item.signal.name || item.signal.code} ${item.signal.direction === "buy_attention" ? "触发买入关注" : "触发卖出关注"}`,
      body: "当前价格进入自动网格触发区，仅供参考。",
    });
  }

  return { sent: eligible.length, denied: false };
}
```

Rules:

- Do not request permission on page load when there are no generated signals.
- Do not send notifications from `recent_signals` or `latest_signal`.
- If denied, return `{ denied: true }` so the page can show a non-blocking warning.

- [ ] **Step 2: Create page skeleton**

Implement `QuantAlertsPage` with Ant Design `Card`, `Alert`, `Button`, `Table`, `Tag`, `Form`, `Input`, `Switch`, and `Modal` as needed.

State requirements:

- `dashboard: QuantDashboard | null`
- `watchlist: QuantWatchlistItem[]`
- `loading`, `refreshing`, `notificationWarning`
- add watchlist form state

- [ ] **Step 3: Implement data loading and one polling interval**

On mount:

- call `api.getQuantDashboard()` and `api.listQuantWatchlist()`.
- immediately call `api.refreshQuantSignals()` once after initial load so eligible signals can persist and notify without waiting for the first interval.
- start exactly one interval based on 60 seconds initially.
- interval calls `api.refreshQuantSignals()`.
- cleanup interval on unmount.
- manual refresh reuses the same refresh function and does not create an interval.

- [ ] **Step 4: Wire notification helper**

After each `api.refreshQuantSignals()` call, call `notifyGeneratedQuantSignals(result.generated_signals)`. If it returns `denied: true`, set a non-blocking warning. Keep in-app history regardless of notification result.

- [ ] **Step 5: Implement status board**

Render target cards/table rows using `target.output_state`, not `latest_signal`, for current status. Use labels:

- `buy_attention` -> `买入关注`
- `sell_attention` -> `卖出关注`
- `watch` -> `观望`
- `quote_error` -> `行情异常`

Show current price, trend state, trigger zone, latest signal time, and error text.

- [ ] **Step 6: Implement target settings controls**

For each target, include switches for:

- `enabled`
- `desktop_notification_enabled`

Both call `api.updateQuantStrategySettings(code, market, input)` and then reload dashboard.

When toggling `enabled`, preserve the current `desktop_notification_enabled` from the target:

```ts
api.updateQuantStrategySettings(target.code, target.market, {
  enabled: nextEnabled,
  desktop_notification_enabled: target.desktop_notification_enabled,
});
```

When toggling `desktop_notification_enabled`, preserve the current `enabled` from the target:

```ts
api.updateQuantStrategySettings(target.code, target.market, {
  enabled: target.enabled,
  desktop_notification_enabled: nextDesktopEnabled,
});
```

- [ ] **Step 7: Implement watchlist management**

Add form with code/name/market. On submit call `api.addQuantWatchlist`. Add delete action for watchlist rows. Do not duplicate held stocks in the board; backend handles merge.

- [ ] **Step 8: Implement signal history with filters**

Render signal history in a table with time, stock, direction, trigger price, source, and trigger zone. Include filters for stock code and direction:

```tsx
const [signalFilter, setSignalFilter] = useState<QuantSignalFilter>({ limit: 100 });
const [signals, setSignals] = useState<QuantSignal[]>([]);

const loadSignals = async (filter = signalFilter) => {
  setSignals(await api.listQuantSignals(filter));
};
```

On initial load, seed `signals` from `dashboard.recent_signals`, then call `loadSignals` when the user changes stock or direction filters. Direction options are `全部`, `买入关注`, and `卖出关注`.

- [ ] **Step 9: Add CSS**

Append focused classes to `global.css`:

```css
.quant-status-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: 16px;
  margin-bottom: 16px;
}

.quant-target-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 16px;
}

.quant-target-card {
  height: 100%;
}

.quant-target-card .ant-card-body {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.quant-target-card__header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}

.quant-target-card__name {
  font-size: 15px;
  font-weight: 600;
  color: var(--color-text);
}

.quant-target-card__code {
  display: block;
  margin-top: 2px;
  font-size: 12px;
  color: var(--color-text-secondary);
}

.quant-target-card__price {
  font-size: 24px;
  font-weight: 700;
  letter-spacing: -0.02em;
}

.quant-target-card__meta {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 8px;
}

.quant-signal-buy { color: var(--color-income); }
.quant-signal-sell { color: var(--color-expense); }
.quant-signal-watch { color: var(--color-text-secondary); }
.quant-signal-error { color: var(--color-expense); }
```

Keep layout responsive using existing `content-grid` patterns where possible.

- [ ] **Step 10: Run frontend typecheck**

Run: `npm run typecheck`

Expected: PASS.

### Task 16: Add Navigation Route

**Files:**
- Modify: `src/App.tsx`

- [ ] **Step 1: Add icon import**

Use an existing Ant Design icon such as `StockOutlined` or `ThunderboltOutlined`.

- [ ] **Step 2: Add menu item and metadata**

Add:

```tsx
{ key: "/quant", icon: <StockOutlined />, label: "量化提醒" }
```

And route metadata:

```tsx
"/quant": { title: "量化提醒", subtitle: "盘中趋势信号与买卖关注提醒" }
```

- [ ] **Step 3: Add route**

Import `QuantAlertsPage` and add:

```tsx
<Route path="/quant" element={<QuantAlertsPage />} />
```

- [ ] **Step 4: Run typecheck**

Run: `npm run typecheck`

Expected: PASS.

- [ ] **Step 5: Commit chunks 4 and 5**

Run only when commits are explicitly requested:

```bash
git add package.json package-lock.json src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/lib.rs src-tauri/capabilities/default.json src/types/index.ts src/api/index.ts src/utils/quantNotifications.ts src/pages/QuantAlerts.tsx src/App.tsx src/styles/global.css
git commit -m "feat(quant): add alert UI"
```

## Chunk 6: End-To-End Verification And Risk Review

### Task 17: Run Full Verification

**Files:**
- No code changes unless failures are found.

- [ ] **Step 1: Backend tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -- --nocapture`

Expected: PASS.

- [ ] **Step 2: Backend compile**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`

Expected: PASS.

- [ ] **Step 3: Frontend typecheck**

Run: `npm run typecheck`

Expected: PASS.

- [ ] **Step 4: Production build**

Run: `npm run build`

Expected: PASS.

- [ ] **Step 5: Manual smoke test**

Run: `npm run tauri dev`

Expected:

- App launches.
- `量化提醒` appears in nav.
- Page loads without crashing.
- Adding a watchlist stock works or returns a friendly API error.
- Enable/disable switches call backend without mutating holdings.
- If strict realtime quote validation fails, target shows `行情异常` and no notification is sent.
- Signal history renders existing rows.

- [ ] **Step 6: Manual desktop notification verification**

Run: `npm run tauri dev`

Expected with notification permission granted:

- Seed or trigger one `generated_signals` item with `desktop_notification_enabled = true`.
- Exactly one desktop notification is shown.
- Existing `recent_signals` do not replay notifications on page reload.

Expected with notification permission denied:

- The signal is still persisted and visible in-app.
- No desktop notification is attempted after denial.
- The page shows a non-blocking warning and does not crash.

Expected with `desktop_notification_enabled = false`:

- A generated signal is persisted and visible in-app.
- No desktop notification is shown.

### Task 18: Final Review Checklist

**Files:**
- Inspect: `git diff`

- [ ] **Step 1: Confirm no forbidden mutation path**

Review `src-tauri/src/commands/quant.rs` and `src-tauri/src/services/quote.rs` to confirm quant refresh does not call `refresh_quotes` and does not update `holdings`, `transactions`, `accounts`, or `price_history`.

- [ ] **Step 2: Confirm notification source**

Review `src/pages/QuantAlerts.tsx` to confirm desktop notifications are sent only from `generated_signals` and only when `desktop_notification_enabled` is true.

- [ ] **Step 3: Confirm stale quote rejection**

Review strict quote validation: missing timestamp/status must produce `quote_error` and no persisted signal.

- [ ] **Step 4: Confirm backup compatibility**

Review `ExportPayload` defaults and `import_data` cleanup order.

- [ ] **Step 5: Commit final fixes**

Run only when commits are explicitly requested:

```bash
git add src-tauri/src/models.rs src-tauri/src/db.rs src-tauri/src/services/quant.rs src-tauri/src/services/mod.rs src-tauri/src/services/quote.rs src-tauri/src/commands/quant.rs src-tauri/src/commands/mod.rs src-tauri/src/commands/data.rs src-tauri/src/lib.rs src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/capabilities/default.json package.json package-lock.json src/types/index.ts src/api/index.ts src/utils/quantNotifications.ts src/pages/QuantAlerts.tsx src/App.tsx src/styles/global.css
git commit -m "test(quant): verify alert module"
```

---

## Handoff Notes

- The strict realtime quote provider is the highest-risk implementation detail. If EastMoney fields do not provide a reliable exchange timestamp or market status, stop and choose a provider that does; do not weaken strict mode silently.
- Keep quant refresh backend-owned. The frontend interval should only call commands and render returned DTOs.
- Do not implement real trading, suggested share quantities, or account/holding mutation in this plan.
- Do not add broad refactors outside files listed above unless a verification failure proves they are necessary.
