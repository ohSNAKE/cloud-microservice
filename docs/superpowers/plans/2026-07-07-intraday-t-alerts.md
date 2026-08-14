# Intraday T Alerts Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a per-target `intraday_t` quant strategy that detects frequent 5-minute high/low time windows and emits `卖T关注` / `买回关注` reminders only when current-day price position confirms the setup.

**Architecture:** Keep strategy calculation backend-owned. Extend the existing quant schema, DTOs, pure Rust rule helpers, and `refresh_quant_signals()` flow so each target evaluates exactly one strategy mode: existing `auto_grid` or new `intraday_t`. The React page remains a polling/rendering client and only displays backend-returned status, reason, windows, and generated signals.

**Tech Stack:** Tauri 2, Rust, rusqlite, chrono, ureq/EastMoney K-line APIs, React 19, TypeScript, Ant Design, Tauri notification plugin.

---

Spec: `docs/superpowers/specs/2026-07-07-intraday-t-alerts-design.md`

## File Structure

- Modify `src-tauri/src/models.rs`: extend quant DTOs and export/import row structs with strategy mode and intraday fields.
- Modify `src-tauri/src/db.rs`: add new `quant_strategy_settings` columns for existing and fresh databases.
- Modify `src-tauri/src/services/quote.rs`: support 5-minute stock K-line period through the existing read-only K-line fetch path.
- Modify `src-tauri/src/services/quant.rs`: add pure intraday T bucket/statistics/decision helpers and unit tests.
- Modify `src-tauri/src/commands/quant.rs`: load strategy settings, fetch minute bars, evaluate selected strategy, persist `intraday_t` signals, and update command tests.
- Modify `src-tauri/src/commands/data.rs`: export/import new quant strategy settings columns.
- Modify `src/types/index.ts`: extend quant union types and target/signal/settings interfaces.
- Modify `src/api/index.ts`: no new command wrappers expected; keep existing `updateQuantStrategySettings()` typed with the expanded input.
- Modify `src/pages/QuantAlerts.tsx`: add strategy selector and intraday T status/history rendering.
- Modify `src/utils/quantNotifications.ts`: add Do T notification labels and body text.
- Modify `src/styles/global.css`: only if existing classes need small additions for intraday status blocks.

## Chunk 1: Schema And Shared Types

### Task 1: Extend Rust Quant Models

**Files:**
- Modify: `src-tauri/src/models.rs:302-354`
- Modify: `src-tauri/src/models.rs:390-403`

- [ ] **Step 1: Update DTOs before implementation use**

Extend existing structs without replacing current fields:

```rust
pub struct QuantStrategySettingsUpdate {
    pub enabled: bool,
    pub desktop_notification_enabled: bool,
    pub strategy_mode: Option<String>,
}

pub struct QuantTarget {
    // existing fields...
    pub strategy_mode: String,
    pub intraday_position: Option<f64>,
    pub intraday_reason: Option<String>,
    pub intraday_high_frequency_windows: Vec<String>,
    pub intraday_low_frequency_windows: Vec<String>,
}

pub struct QuantGeneratedSignal {
    // existing fields...
    pub notification_body: Option<String>,
}

pub struct QuantStrategySettingsRow {
    // existing fields...
    #[serde(default = "default_quant_strategy_mode")]
    pub strategy_mode: String,
    #[serde(default = "default_intraday_lookback_days")]
    pub intraday_lookback_days: i64,
    #[serde(default = "default_intraday_time_min_count")]
    pub intraday_high_time_min_count: i64,
    #[serde(default = "default_intraday_time_min_count")]
    pub intraday_low_time_min_count: i64,
    #[serde(default = "default_sell_t_position_threshold")]
    pub sell_t_position_threshold: f64,
    #[serde(default = "default_buyback_position_threshold")]
    pub buyback_position_threshold: f64,
}
```

Add the default functions near the struct definitions so old backups with quant settings rows but no intraday columns still import:

```rust
fn default_quant_strategy_mode() -> String { "auto_grid".to_string() }
fn default_intraday_lookback_days() -> i64 { 22 }
fn default_intraday_time_min_count() -> i64 { 4 }
fn default_sell_t_position_threshold() -> f64 { 0.70 }
fn default_buyback_position_threshold() -> f64 { 0.30 }
```

- [ ] **Step 2: Update every `QuantTarget` construction**

In `src-tauri/src/commands/quant.rs`, initialize new fields in `quant_target_from_candidate_settings()`:

```rust
strategy_mode: "auto_grid".to_string(),
intraday_position: None,
intraday_reason: None,
intraday_high_frequency_windows: vec![],
intraday_low_frequency_windows: vec![],
```

- [ ] **Step 3: Run compile check**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`

Expected: failures only where settings/export/import SQL does not yet populate new fields.

### Task 2: Add Strategy Columns To SQLite

**Files:**
- Modify: `src-tauri/src/db.rs:111-124`
- Modify: `src-tauri/src/db.rs:204-217`
- Modify: `src-tauri/src/db.rs:153-245`
- Modify: `src-tauri/src/commands/quant.rs:960-1055`

- [ ] **Step 1: Add columns to fresh table SQL**

In both `init_db()` and `migrate()` `CREATE TABLE IF NOT EXISTS quant_strategy_settings`, add:

```sql
strategy_mode TEXT NOT NULL DEFAULT 'auto_grid',
intraday_lookback_days INTEGER NOT NULL DEFAULT 22,
intraday_high_time_min_count INTEGER NOT NULL DEFAULT 4,
intraday_low_time_min_count INTEGER NOT NULL DEFAULT 4,
sell_t_position_threshold REAL NOT NULL DEFAULT 0.70,
buyback_position_threshold REAL NOT NULL DEFAULT 0.30,
```

- [ ] **Step 2: Add idempotent migration for existing tables**

Add a small helper near `migrate()`:

```rust
fn ensure_column(
    conn: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<(), rusqlite::Error> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let exists = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .filter_map(Result::ok)
        .any(|name| name == column);
    if !exists {
        conn.execute(&format!("ALTER TABLE {table} ADD COLUMN {definition}"), [])?;
    }
    Ok(())
}
```

After the existing `CREATE TABLE IF NOT EXISTS` batch in `migrate()`, call it for each new column:

```rust
ensure_column(conn, "quant_strategy_settings", "strategy_mode", "strategy_mode TEXT NOT NULL DEFAULT 'auto_grid'")?;
ensure_column(conn, "quant_strategy_settings", "intraday_lookback_days", "intraday_lookback_days INTEGER NOT NULL DEFAULT 22")?;
ensure_column(conn, "quant_strategy_settings", "intraday_high_time_min_count", "intraday_high_time_min_count INTEGER NOT NULL DEFAULT 4")?;
ensure_column(conn, "quant_strategy_settings", "intraday_low_time_min_count", "intraday_low_time_min_count INTEGER NOT NULL DEFAULT 4")?;
ensure_column(conn, "quant_strategy_settings", "sell_t_position_threshold", "sell_t_position_threshold REAL NOT NULL DEFAULT 0.70")?;
ensure_column(conn, "quant_strategy_settings", "buyback_position_threshold", "buyback_position_threshold REAL NOT NULL DEFAULT 0.30")?;
```

- [ ] **Step 3: Update test schema**

Mirror the same columns in `create_test_schema()` in `src-tauri/src/commands/quant.rs`.

- [ ] **Step 4: Run schema-adjacent tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml quant::tests::dashboard_does_not_insert_missing_strategy_settings -- --nocapture`

Expected: PASS after model construction and test schema are updated.

### Task 3: Update Export And Import Rows

**Files:**
- Modify: `src-tauri/src/commands/data.rs`
- Modify: `src-tauri/src/models.rs:390-403`

- [ ] **Step 1: Write/adjust compatibility tests first**

Extend existing export/import tests so `quant_strategy_settings` rows include the new fields. Add a separate test where an old backup includes `quant_strategy_settings` rows that omit only the new intraday fields; it must deserialize and use defaults: `auto_grid`, `22`, `4`, `4`, `0.70`, `0.30`.

Run: `cargo test --manifest-path src-tauri/Cargo.toml quant -- --nocapture`

Run: `cargo test --manifest-path src-tauri/Cargo.toml export -- --nocapture`

Run: `cargo test --manifest-path src-tauri/Cargo.toml import -- --nocapture`

Expected: FAIL until `data.rs` maps the new columns.

- [ ] **Step 2: Update export SELECT for settings**

Select the new fields from `quant_strategy_settings`:

```sql
SELECT id, code, market, ma_short, ma_long, grid_lookback_days,
       poll_interval_seconds, desktop_notification_enabled, enabled,
       strategy_mode, intraday_lookback_days, intraday_high_time_min_count,
       intraday_low_time_min_count, sell_t_position_threshold,
       buyback_position_threshold, created_at, updated_at
FROM quant_strategy_settings
ORDER BY id
```

- [ ] **Step 3: Update import INSERT for settings**

Insert explicit values for all new fields. Preserve IDs. Convert booleans to `0`/`1` as current code does.

- [ ] **Step 4: Run backend checks**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`

Expected: PASS.

Run: `cargo test --manifest-path src-tauri/Cargo.toml export_payload_accepts_missing_quant_fields -- --nocapture`

Expected: PASS.

## Chunk 2: Pure Intraday T Rules

### Task 4: Add 5-Minute Time-Bucket Helpers

**Files:**
- Modify: `src-tauri/src/services/quant.rs:1-249`
- Test: `src-tauri/src/services/quant.rs:250-622`

- [ ] **Step 1: Write failing bucket tests**

Add tests:

```rust
#[test]
fn intraday_t_assigns_time_buckets() {
    assert_eq!(intraday_t_bucket("2026-07-07 09:35").as_deref(), Some("09:30-09:45"));
    assert_eq!(intraday_t_bucket("2026-07-07 10:00").as_deref(), Some("09:50-10:30"));
    assert_eq!(intraday_t_bucket("2026-07-07 14:50").as_deref(), Some("14:35-15:00"));
    assert_eq!(intraday_t_bucket("2026-07-07 11:45"), None);
}

#[test]
fn intraday_t_groups_by_date_prefix() {
    let bars = vec![minute_bar("2026-07-01 09:35", 10.0, 11.0, 9.0), minute_bar("2026-07-01 10:00", 10.0, 10.5, 8.5)];
    let grouped = group_intraday_bars_by_day(&bars);
    assert_eq!(grouped.len(), 1);
    assert_eq!(grouped[0].0, "2026-07-01");
}
```

Expected: FAIL because helpers do not exist.

- [ ] **Step 2: Implement minimal helpers**

Add near existing pure helpers:

```rust
pub const INTRADAY_T_BUCKETS: [&str; 6] = [
    "09:30-09:45",
    "09:50-10:30",
    "10:35-11:30",
    "13:00-13:30",
    "13:35-14:30",
    "14:35-15:00",
];

pub fn intraday_t_bucket(date_time: &str) -> Option<String> {
    let time = date_time.split_whitespace().nth(1)?;
    let time = chrono::NaiveTime::parse_from_str(time, "%H:%M")
        .or_else(|_| chrono::NaiveTime::parse_from_str(time, "%H:%M:%S"))
        .ok()?;
    let bucket = |start, end, label| {
        let start = chrono::NaiveTime::parse_from_str(start, "%H:%M").unwrap();
        let end = chrono::NaiveTime::parse_from_str(end, "%H:%M").unwrap();
        (time >= start && time <= end).then(|| label.to_string())
    };
    bucket("09:30", "09:45", "09:30-09:45")
        .or_else(|| bucket("09:50", "10:30", "09:50-10:30"))
        .or_else(|| bucket("10:35", "11:30", "10:35-11:30"))
        .or_else(|| bucket("13:00", "13:30", "13:00-13:30"))
        .or_else(|| bucket("13:35", "14:30", "13:35-14:30"))
        .or_else(|| bucket("14:35", "15:00", "14:35-15:00"))
}
```

Implement date-prefix grouping with `bar.date.split_whitespace().next()`.

- [ ] **Step 3: Run tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml intraday_t_ -- --nocapture`

Expected: PASS.

### Task 5: Add Historical Frequency Statistics

**Files:**
- Modify: `src-tauri/src/services/quant.rs`

- [ ] **Step 1: Write failing statistics tests**

Add tests that build at least 10 completed days of 5-minute bars:

```rust
#[test]
fn intraday_t_detects_high_and_low_frequency_windows() {
    let bars = intraday_t_history_with_repeated_highs_and_lows();
    let stats = analyze_intraday_t_windows(&bars, "2026-07-07", 22, 4, 4);
    assert_eq!(stats.valid_days, 10);
    assert!(stats.high_frequency_windows.contains(&"09:30-09:45".to_string()));
    assert!(stats.low_frequency_windows.contains(&"13:35-14:30".to_string()));
}

#[test]
fn intraday_t_excludes_current_day_from_frequency_stats() {
    let mut bars = intraday_t_history_with_repeated_highs_and_lows();
    bars.push(minute_bar("2026-07-07 09:35", 10.0, 99.0, 9.0));
    let stats = analyze_intraday_t_windows(&bars, "2026-07-07", 22, 1, 1);
    assert!(!stats.high_frequency_windows.iter().any(|bucket| bucket == "09:30-09:45" && stats.valid_days == 11));
}
```

Expected: FAIL.

- [ ] **Step 2: Implement stats structs and function**

Add:

```rust
#[derive(Debug, Clone, Default, PartialEq)]
pub struct IntradayTWindowStats {
    pub valid_days: usize,
    pub high_frequency_windows: Vec<String>,
    pub low_frequency_windows: Vec<String>,
}

pub fn analyze_intraday_t_windows(
    bars: &[crate::models::KlineBar],
    current_date: &str,
    lookback_days: usize,
    high_min_count: usize,
    low_min_count: usize,
) -> IntradayTWindowStats {
    // Group by completed date, exclude current_date, keep latest lookback_days.
    // For each day, find max high bar and min low bar, then count their buckets.
    // Return bucket labels in INTRADAY_T_BUCKETS order when counts meet thresholds.
}
```

Keep the function pure and deterministic. A completed historical day is valid only when it has enough bucket coverage to avoid treating a partial or corrupt day as a real trading day. Use at least 36 bars, which is about 75% of a CN 5-minute session, and require at least one bar in the morning and one in the afternoon. If a day's high or low time is outside known buckets, do not count that occurrence, but the day can still count as valid if it meets the coverage rule.

Add a test that a partial historical day with only one or two bars does not count toward `valid_days` or frequency thresholds.

- [ ] **Step 3: Run statistics tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml intraday_t_ -- --nocapture`

Expected: PASS.

### Task 6: Add Current-Day Position And Decision Rules

**Files:**
- Modify: `src-tauri/src/services/quant.rs`

- [ ] **Step 1: Write failing decision tests**

Add tests:

```rust
#[test]
fn intraday_t_calculates_current_day_position() {
    let bars = vec![minute_bar("2026-07-07 09:35", 10.0, 12.0, 8.0)];
    let position = intraday_t_day_position(&bars, "2026-07-07", 11.0, 0.0001).unwrap();
    assert!((position - 0.75).abs() < 1e-9);
}

#[test]
fn intraday_t_rejects_narrow_current_day_range() {
    let bars = vec![minute_bar("2026-07-07 09:35", 10.0, 10.00001, 10.0)];
    assert!(intraday_t_day_position(&bars, "2026-07-07", 10.0, 0.0001).is_none());
}

#[test]
fn intraday_t_emits_sell_and_buyback_from_time_plus_position() {
    let stats = IntradayTWindowStats { valid_days: 22, high_frequency_windows: vec!["09:30-09:45".into()], low_frequency_windows: vec!["13:35-14:30".into()] };
    let sell = decide_intraday_t_signal(cn_datetime(2026, 7, 7, 9, 35, 0), 0.76, &stats, 0.70, 0.30);
    assert_eq!(sell.output_state, "sell_t_attention");
    assert_eq!(sell.current_trigger_zone.as_deref(), Some("09:30-09:45"));
    assert!(sell.reason.unwrap().contains("历史"));

    let buyback = decide_intraday_t_signal(cn_datetime(2026, 7, 7, 13, 40, 0), 0.24, &stats, 0.70, 0.30);
    assert_eq!(buyback.output_state, "buyback_attention");
    assert_eq!(buyback.current_trigger_zone.as_deref(), Some("13:35-14:30"));
}
```

Expected: FAIL.

- [ ] **Step 2: Implement decision helpers**

Add a decision result that reuses existing fields plus reason:

```rust
pub struct IntradayTDecision {
    pub output_state: String,
    pub current_trigger_zone: Option<String>,
    pub reason: Option<String>,
}
```

Implement:

```rust
pub fn intraday_t_day_position(
    bars: &[crate::models::KlineBar],
    current_date: &str,
    current_price: f64,
    min_range: f64,
) -> Option<f64> { /* use only current_date prefix, include current_price in high/low range */ }

pub fn decide_intraday_t_signal(
    now: chrono::DateTime<chrono::FixedOffset>,
    position: f64,
    stats: &IntradayTWindowStats,
    sell_threshold: f64,
    buyback_threshold: f64,
) -> IntradayTDecision { /* apply bucket plus threshold rules */ }
```

If both sell and buyback conditions match, return `watch` with a defensive reason.

- [ ] **Step 3: Run rule tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml intraday_t_ -- --nocapture`

Expected: all pure intraday T tests PASS.

## Chunk 3: Backend Integration

### Task 7: Fetch Minute K-Lines For Intraday T Targets

**Files:**
- Modify: `src-tauri/src/services/quote.rs:168-190`
- Modify: `src-tauri/src/commands/quant.rs:19-31`
- Modify: `src-tauri/src/commands/quant.rs:734-765`

- [ ] **Step 1: Add failing quote period test**

Add a test in `quote.rs`:

```rust
#[test]
fn stock_kline_period_supports_five_minute() {
    assert_eq!(kline_period_code("5m"), 5);
}
```

Expected: FAIL because unknown periods default to day.

- [ ] **Step 2: Implement `5m` period**

Update `kline_period_code()`:

```rust
"5m" | "5min" => 5,
```

- [ ] **Step 3: Extend snapshots**

Change `QuantMarketSnapshot`:

```rust
pub struct QuantMarketSnapshot {
    code: String,
    market: String,
    daily_bars: Result<Vec<KlineBar>, String>,
    minute_bars: Result<Vec<KlineBar>, String>,
    realtime_quote: Result<RealtimeStockQuote, String>,
}
```

Update all test snapshot constructors to include `minute_bars`, usually `Ok(vec![])` for auto-grid tests.

- [ ] **Step 4: Fetch minute bars only when needed**

Extend settings load to include `strategy_mode` and intraday lookback. In `load_live_quant_snapshots()`, if `strategy_mode == "intraday_t"`, fetch:

```rust
fetch_stock_kline(&code, "5m", (settings.intraday_lookback_days * 60) as i64).await
```

Use a generous limit because one CN trading day has about 48 5-minute bars. Keep auto-grid targets from fetching minute bars by setting `minute_bars = Ok(vec![])`.

- [ ] **Step 5: Run quote test and compile**

Run: `cargo test --manifest-path src-tauri/Cargo.toml stock_kline_period_supports_five_minute -- --nocapture`

Expected: PASS.

Run: `cargo check --manifest-path src-tauri/Cargo.toml`

Expected: PASS after constructor updates.

### Task 8: Load And Save Strategy Mode Settings

**Files:**
- Modify: `src-tauri/src/commands/quant.rs:43-80`
- Modify: `src-tauri/src/commands/quant.rs:191-223`
- Modify: `src-tauri/src/commands/quant.rs:372-407`
- Test: `src-tauri/src/commands/quant.rs:2051-2133`

- [ ] **Step 1: Write failing settings tests**

Add tests:

```rust
#[test]
fn settings_default_to_auto_grid_strategy_mode() {
    let conn = test_conn();
    insert_watchlist(&conn, "000008", "Default", "cn", true);
    let targets = merge_quant_targets(&conn).unwrap();
    assert_eq!(targets[0].strategy_mode, "auto_grid");
}

#[test]
fn update_settings_persists_intraday_t_strategy_mode() {
    let conn = test_conn();
    insert_watchlist(&conn, "000009", "Do T", "cn", true);
    let target = update_quant_strategy_settings_in_conn(&conn, "000009".into(), "cn".into(), QuantStrategySettingsUpdate {
        enabled: true,
        desktop_notification_enabled: true,
        strategy_mode: Some("intraday_t".into()),
    }).unwrap();
    assert_eq!(target.strategy_mode, "intraday_t");
}
```

Expected: FAIL until settings are loaded/saved.

- [ ] **Step 2: Extend internal settings row**

Add fields to the private `QuantStrategySettingsRow` in `commands/quant.rs`:

```rust
strategy_mode: String,
intraday_lookback_days: usize,
intraday_high_time_min_count: usize,
intraday_low_time_min_count: usize,
sell_t_position_threshold: f64,
buyback_position_threshold: f64,
```

Load them in `load_quant_strategy_settings_row()`, defaulting to spec values when no row exists.

- [ ] **Step 3: Return strategy mode on targets**

Update `ensure_quant_settings()` and read-only target construction to read `strategy_mode` along with enabled flags. Set `target.strategy_mode` before returning.

- [ ] **Step 4: Save validated strategy mode**

In `update_quant_strategy_settings_in_conn()`, validate `input.strategy_mode.unwrap_or_else(|| existing_or_default)` is `auto_grid` or `intraday_t`. Add it to the UPSERT:

```sql
strategy_mode = excluded.strategy_mode,
```

Do not reset numeric intraday defaults when toggling enabled/notification.

- [ ] **Step 5: Run settings tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml strategy_mode -- --nocapture`

Expected: PASS.

### Task 9: Evaluate Intraday T In Dashboard And Refresh

**Files:**
- Modify: `src-tauri/src/commands/quant.rs:506-601`
- Modify: `src-tauri/src/commands/quant.rs:643-682`
- Modify: `src-tauri/src/commands/quant.rs:684-732`
- Test: `src-tauri/src/commands/quant.rs:1366-1906`

- [ ] **Step 1: Write failing integration tests**

Add tests with minute-bar fixtures:

```rust
#[test]
fn intraday_t_dashboard_emits_sell_attention_with_reason_and_position() { /* target strategy_mode intraday_t, now 09:35, repeated high bucket, position >= .70 */ }

#[test]
fn intraday_t_dashboard_emits_buyback_attention_with_reason_and_position() { /* now 13:40, repeated low bucket, position <= .30 */ }

#[test]
fn intraday_t_insufficient_history_stays_watch() { /* fewer than 10 completed days */ }

#[test]
fn intraday_t_narrow_current_range_stays_watch() { /* current day high-low below epsilon */ }

#[test]
fn intraday_t_watch_outcomes_include_display_reason() { /* insufficient history, narrow range, ordinary watch, and contradictory conditions all set intraday_reason */ }

#[test]
fn intraday_t_weak_trend_does_not_block_sell_t() { /* bearish daily trend but valid sell setup */ }

#[test]
fn intraday_t_refresh_persists_intraday_signal_source_and_bucket() { /* source intraday_t, trigger_zone time bucket */ }

#[test]
fn intraday_t_refresh_rejects_stale_or_non_realtime_quote() { /* output quote_error, no persisted signal */ }

#[test]
fn intraday_t_market_closed_persists_no_signal() { /* non-trading time returns generated_signals empty */ }

#[test]
fn intraday_t_cooldown_dedupes_same_direction_and_bucket() { /* same stock/market/direction/time bucket within 5 minutes is suppressed */ }

#[test]
fn intraday_t_cooldown_allows_different_direction_or_bucket() { /* sell_t and buyback, or distinct buckets, do not block each other */ }

#[test]
fn intraday_t_contradictory_conditions_stay_watch() { /* defensive no contradictory advice */ }

#[test]
fn intraday_t_and_auto_grid_modes_are_isolated() { /* one target auto_grid, one intraday_t, each evaluates only selected mode */ }
```

Expected: FAIL until dashboard and refresh paths branch by strategy.

- [ ] **Step 2: Add intraday evaluator inside dashboard build**

In `build_quant_dashboard_with_snapshots()` after trend display calculation, branch:

```rust
if settings.strategy_mode == "intraday_t" {
    target.grid_zones = vec![];
    target.strategy_mode = "intraday_t".to_string();
    // Use snapshot.minute_bars, analyze windows, compute current-day position, decide signal.
    continue;
}

// existing auto_grid path remains unchanged
```

For intraday T:

```rust
let current_date = now.format("%Y-%m-%d").to_string();
let minute_bars = snapshot.minute_bars.as_ref().map_err(...);
let stats = analyze_intraday_t_windows(minute_bars, &current_date, settings.intraday_lookback_days, settings.intraday_high_time_min_count, settings.intraday_low_time_min_count);
target.intraday_high_frequency_windows = stats.high_frequency_windows.clone();
target.intraday_low_frequency_windows = stats.low_frequency_windows.clone();
```

Apply boundaries in this order: insufficient history, narrow range, decision. Always set `target.intraday_reason` for intraday T targets, not only triggered signals:

```rust
target.intraday_reason = Some("分时统计数据不足".to_string());
target.intraday_reason = Some("当日波动不足，暂不触发".to_string());
target.intraday_reason = decision.reason.or_else(|| Some("未进入高发时段或日内位置未达阈值".to_string()));
```

Triggered outcomes should include action plus reason, for example `卖T关注 · 历史早盘高点高发` or `买回关注 · 午后低点高发`. Contradictory outcomes should stay `watch` and set a defensive reason.

- [ ] **Step 3: Generalize signal insertion**

Rename `insert_auto_quant_signal()` to `insert_quant_signal()` and accept `source: &str`. Use `target.output_state` for `sell_t_attention` and `buyback_attention` as well.

When pushing `QuantGeneratedSignal`, set `notification_body` from the target state so desktop notifications can include the time bucket and day-position percentage without persisting those display details in `quant_signals`:

```rust
let notification_body = if source == "intraday_t" {
    target.intraday_position.map(|position| {
        format!(
            "历史高发时段 {}，当前日内位置 {:.0}%，仅供参考。",
            trigger_zone,
            position * 100.0
        )
    })
} else {
    None
};
```

- [ ] **Step 4: Persist new directions**

In `refresh_quant_signals_with_snapshots()`, replace the output-state filter with:

```rust
if !matches!(target.output_state.as_str(), "buy_attention" | "sell_attention" | "sell_t_attention" | "buyback_attention") {
    continue;
}
let source = if target.strategy_mode == "intraday_t" { "intraday_t" } else { "auto_grid" };
```

The dedupe key already includes direction and trigger zone, so it works for time buckets.

- [ ] **Step 5: Run focused backend tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml intraday_t_ -- --nocapture`

Expected: PASS.

Run: `cargo test --manifest-path src-tauri/Cargo.toml refresh_ -- --nocapture`

Expected: existing auto-grid tests still PASS.

## Chunk 4: Frontend And Notifications

### Task 10: Extend TypeScript Types

**Files:**
- Modify: `src/types/index.ts:274-348`

- [ ] **Step 1: Expand unions and interfaces**

Update types:

```ts
export type QuantStrategyMode = "auto_grid" | "intraday_t";
export type QuantDirection = "buy_attention" | "sell_attention" | "sell_t_attention" | "buyback_attention";
export type QuantTimeBucket = "09:30-09:45" | "09:50-10:30" | "10:35-11:30" | "13:00-13:30" | "13:35-14:30" | "14:35-15:00";
export type QuantTriggerZone = "buy_1" | "buy_2" | "sell_1" | "sell_2" | QuantTimeBucket;

export interface QuantStrategySettingsUpdate {
  enabled: boolean;
  desktop_notification_enabled: boolean;
  strategy_mode?: QuantStrategyMode;
}

export interface QuantTarget {
  strategy_mode: QuantStrategyMode;
  intraday_position: number | null;
  intraday_reason: string | null;
  intraday_high_frequency_windows: QuantTimeBucket[];
  intraday_low_frequency_windows: QuantTimeBucket[];
}

export interface QuantSignal {
  source: "auto_grid" | "intraday_t";
  trigger_zone: QuantTriggerZone;
}

export interface QuantGeneratedSignal {
  signal: QuantSignal;
  desktop_notification_enabled: boolean;
  notification_body: string | null;
}
```

Keep all existing fields on `QuantTarget`, `QuantSignal`, and `QuantGeneratedSignal`; the snippets above show only fields that change or are added.

- [ ] **Step 2: Run typecheck**

Run: `npm run typecheck`

Expected: FAIL until page and notifications handle new union members.

### Task 11: Update Quant Page UI

**Files:**
- Modify: `src/pages/QuantAlerts.tsx:44-68`
- Modify: `src/pages/QuantAlerts.tsx:237-274`
- Modify: `src/pages/QuantAlerts.tsx:397-467`
- Modify: `src/pages/QuantAlerts.tsx:552-596`

- [ ] **Step 1: Update labels**

Extend maps:

```ts
const outputLabels: Record<QuantOutputState, string> = {
  buy_attention: "买入关注",
  sell_attention: "卖出关注",
  sell_t_attention: "卖T关注",
  buyback_attention: "买回关注",
  watch: "观望",
  quote_error: "行情异常",
};

const directionLabels: Record<QuantDirection, string> = {
  buy_attention: "买入关注",
  sell_attention: "卖出关注",
  sell_t_attention: "卖T关注",
  buyback_attention: "买回关注",
};
```

Use `sourceLabel()` to map `intraday_t` to `做T`.

Extend `outputClasses` for the new output states. `sell_t_attention` can reuse the sell class and `buyback_attention` can reuse the buy class:

```ts
sell_t_attention: "quant-signal-sell",
buyback_attention: "quant-signal-buy",
```

- [ ] **Step 2: Preserve strategy mode on settings updates**

Change `updateTargetSettings()` input to include optional `strategy_mode`, and always send the current mode when toggling enabled/desktop reminder:

```ts
strategy_mode: nextSettings.strategy_mode ?? target.strategy_mode,
```

- [ ] **Step 3: Add strategy selector**

In each target card, add a `Select` with options `自动网格` and `做T`. On change call `updateTargetSettings(target, { enabled: target.enabled, desktop_notification_enabled: target.desktop_notification_enabled, strategy_mode: value })`.

- [ ] **Step 4: Render intraday T details only for Do T targets**

When `target.strategy_mode === "intraday_t"`, show:

```tsx
<div className="quant-target-card__meta">
  <span>日内位置：{target.intraday_position === null ? "--" : `${Math.round(target.intraday_position * 100)}%`}</span>
  <span>卖T窗口：{target.intraday_high_frequency_windows.join("、") || "--"}</span>
  <span>买回窗口：{target.intraday_low_frequency_windows.join("、") || "--"}</span>
  <span>理由：{target.intraday_reason ?? "--"}</span>
</div>
```

For auto-grid targets, keep the existing trigger-zone display.

- [ ] **Step 5: Update history filter and renderers**

Add direction filter options for `卖T关注` and `买回关注`. Change `triggerZoneLabels` from `Record<QuantTriggerZone, string>` to `Partial<Record<QuantTriggerZone, string>>` before using a fallback. Render time-bucket trigger zones directly when `triggerZoneLabels[value]` is missing:

```ts
triggerZoneLabels[value] ?? value
```

- [ ] **Step 6: Run typecheck**

Run: `npm run typecheck`

Expected: remaining failures only in notifications if not updated yet.

### Task 12: Update Notifications

**Files:**
- Modify: `src/utils/quantNotifications.ts:28-34`

- [ ] **Step 1: Add direction/source-specific text**

Replace the two-state direction label with:

```ts
const directionLabels: Record<string, string> = {
  buy_attention: "买入关注",
  sell_attention: "卖出关注",
  sell_t_attention: "卖T关注",
  buyback_attention: "买回关注",
};

for (const item of eligibleSignals) {
  const { signal } = item;
  const title = `量化提醒：${signal.name || signal.code} ${directionLabels[signal.direction] ?? signal.direction}`;

  const body = item.notification_body ?? (signal.source === "intraday_t"
    ? `历史高发时段 ${signal.trigger_zone}，当前日内位置已触发阈值，仅供参考。`
    : "当前价格进入自动网格触发区，仅供参考。");

  sendNotification({ title, body });
}
```

- [ ] **Step 2: Run frontend checks**

Run: `npm run typecheck`

Expected: PASS.

Run: `npm run build`

Expected: PASS.

## Chunk 5: Final Verification

### Task 13: Full Backend Verification

**Files:**
- Verify only, no edits unless failures reveal defects.

- [ ] **Step 1: Run full Rust tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -- --nocapture`

Expected: PASS.

- [ ] **Step 2: Run Rust compile check**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`

Expected: PASS.

### Task 14: Full Frontend Verification

**Files:**
- Verify only, no edits unless failures reveal defects.

- [ ] **Step 1: Typecheck**

Run: `npm run typecheck`

Expected: PASS.

- [ ] **Step 2: Production build**

Run: `npm run build`

Expected: PASS.

### Task 15: Manual Smoke Checks

**Files:**
- Verify behavior in the running app.

- [ ] **Step 1: Open `量化提醒` page**

Expected: existing auto-grid targets still render.

- [ ] **Step 2: Switch one target to `做T`**

Expected: strategy selector saves and the card shows day-position, reason, and high/low windows when data is available.

- [ ] **Step 3: Confirm signal history labels**

Expected: `卖T关注` / `买回关注` display with source `做T` and trigger zone as a time bucket.

- [ ] **Step 4: Confirm no financial table mutation**

Expected: holdings, accounts, transactions, and price history are unchanged except quant signal history.
