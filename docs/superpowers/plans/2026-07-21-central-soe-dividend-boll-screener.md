# Central SOE Dividend BOLL Screener Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a local A-share and BSE screener for central SOEs with CNY 50 billion market capitalization, at least 5% preceding-year dividend yield, and a current price within 2% of the daily or weekly BOLL(20, 2) lower band.

**Architecture:** A new Rust screener domain service performs deterministic eligibility and BOLL calculations from normalized source records. A Tauri command layer persists derived snapshots and run results in SQLite, while a React page owns the visible 15-minute refresh timer and renders the persisted dashboard. Existing quant watchlist code remains a one-way integration target only.

**Tech Stack:** Rust, Tauri 2, rusqlite, chrono, ureq, serde, React 19, TypeScript, Ant Design, Vitest.

---

## File Structure

- Create: `src-tauri/src/services/screener.rs` - Pure ownership, dividend, bar-validation, BOLL, candidate, and schedule calculations with unit tests.
- Create: `src-tauri/src/services/screener_source.rs` - Small source-agnostic records and the `ScreenerSource` trait consumed by refresh orchestration.
- Create: `src-tauri/src/services/screener_source_eastmoney_universe.rs` - Eastmoney universe pagination and actual-controller profile parsing with fixture tests.
- Create: `src-tauri/src/services/screener_source_eastmoney_market.rs` - Eastmoney quote, unadjusted K-line, dividend, and share-capital parsing with fixture tests.
- Create: `src-tauri/src/services/screener_source_sasac.rs` - SASAC directory fetching, controller-name/alias extraction, and fixture tests.
- Create: `src-tauri/resources/sasac_controller_aliases.json` - Versioned historical controller aliases with official SASAC source URLs and announcement dates.
- Create: `src-tauri/src/commands/screener.rs` - Thin Tauri command handlers and one-way quant-watchlist action.
- Create: `src-tauri/src/commands/screener_repository.rs` - SQLite snapshot/result persistence, dashboard reads, pruning, and repository tests.
- Create: `src-tauri/src/commands/screener_refresh.rs` - Single-flight staged refresh coordination and refresh tests.
- Create: `src-tauri/src/services/screener_calendar.rs` - China exchange session/holiday schedule calculation with unit tests.
- Create: `src-tauri/resources/china_market_holidays.json` - Versioned exchange closure dates used by the calendar service.
- Modify: `src-tauri/src/db.rs` - Create derived screener tables, indexes, and generation setting.
- Modify: `src-tauri/src/models.rs` - Serde DTOs shared by commands and the TypeScript API.
- Modify: `src-tauri/src/services/mod.rs` - Expose screener modules.
- Modify: `src-tauri/src/commands/mod.rs` - Expose screener commands.
- Modify: `src-tauri/src/lib.rs` - Register screener state and Tauri commands.
- Modify: `src-tauri/src/commands/data.rs` - Clear derived screener tables and increment generation during imports.
- Modify: `src-tauri/src/services/quote.rs` - Support canonical Shanghai, Shenzhen, and Beijing exchange security IDs and unadjusted daily/weekly bars used by the screener.
- Create: `src/pages/Screener.tsx` - Dashboard lifecycle, refresh scheduling, result table, K-line modal, and quant-watchlist action.
- Create: `src/pages/screenerView.ts` - Pure frontend formatting, row sorting, and state helpers with Vitest coverage.
- Create: `src/pages/screenerView.test.ts` - Tests for sorting, null period values, and stale/error state helpers.
- Create: `src/pages/Screener.test.tsx` - Rendered-page state, manual refresh, scheduling, and watchlist integration tests.
- Modify: `package.json` - Add DOM test dependencies and a test setup entry.
- Create: `src/test/setup.ts` - Extend Vitest with DOM cleanup and Tauri API mocks.
- Modify: `vite.config.ts` - Configure jsdom test environment and setup file.
- Modify: `src/types/index.ts` - Frontend screener DTOs and discriminated status types.
- Modify: `src/api/index.ts` - Tauri invoke wrappers for the screener commands and idempotent quant-watchlist add.
- Modify: `src/App.tsx` - Sidebar item, route, title, and subtitle.
- Modify: `src/styles/global.css` - Dense, responsive status and results-table styling.

## Preflight

- [ ] **Step 1: Capture the implementation baseline before changing source code.**

Run: `git rev-parse HEAD > /var/folders/9l/4ybk4rnn3255lfv5vvnjnp340000gn/T/opencode/screener-base-head.txt && git status --short > /var/folders/9l/4ybk4rnn3255lfv5vvnjnp340000gn/T/opencode/screener-status-before.txt`

Expected: The pre-existing branch commit and dirty-worktree entries are recorded so they are excluded from the feature review.

## Chunk 1: Deterministic Market Rules And Source Normalization

### Task 1: Add the failing screener-domain tests

**Files:**
- Create: `src-tauri/src/services/screener.rs`
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: Write tests for controller normalization and authoritative matching.**

```rust
#[test]
fn central_controller_match_is_exact_after_allowed_suffix_normalization() {
    let registry = CentralControllerRegistry::from_entries(vec![
        CentralControllerEntry::new("中国移动通信集团有限公司", vec!["中国移动".into()]),
    ]);

    assert!(is_central_soe("中国移动通信集团", &registry));
    assert!(is_central_soe("国务院国有资产监督管理委员会", &registry));
    assert!(is_central_soe("国务院", &registry));
    assert!(!is_central_soe("某地方国资委控股的中国移动通信集团", &registry));
}
```

- [ ] **Step 2: Write tests for trailing dividend yield and adjustment rejection.**

```rust
#[test]
fn dividend_yield_uses_gross_current_share_dividends_from_prior_calendar_year() {
    let dividends = vec![
        dividend("2025-06-01", 0.5, DividendAdjustment::Adjusted),
        dividend("2025-12-20", 0.7, DividendAdjustment::VerifiedUnadjusted),
    ];

    assert_eq!(trailing_cash_dividend(&dividends, 2025), Some(1.2));
    assert_eq!(dividend_yield(&dividends, 2025, 20.0), Some(0.06));
}

#[test]
fn unverifiable_confirmed_cash_dividend_invalidates_the_stock() {
    let dividends = vec![dividend("2025-06-01", 0.5, DividendAdjustment::Unverifiable)];
    assert_eq!(trailing_cash_dividend(&dividends, 2025), None);
}

#[test]
fn fundamental_thresholds_include_exact_fifty_billion_and_five_percent() {
    assert!(passes_fundamentals(50_000_000_000.0, 0.05));
    assert!(!passes_fundamentals(49_999_999_999.99, 0.05));
    assert!(!passes_fundamentals(50_000_000_000.0, 0.049_999));
}

#[test]
fn unconfirmed_non_cash_or_missing_date_records_do_not_add_to_cash_dividend() {
    let dividends = vec![unconfirmed_cash(), stock_dividend(), missing_ex_date()];
    assert_eq!(trailing_cash_dividend(&dividends, 2025), None);
}
```

- [ ] **Step 3: Write tests for `BOLL(20, 2)` and completed-bar selection.**

```rust
#[test]
fn boll_uses_population_deviation_and_excludes_today_before_1500() {
    let mut bars = test_bars(20, 10.0);
    bars.push(bar("2026-07-21", 8.0));
    let now = china_datetime(2026, 7, 21, 14, 59, 59);

    let result = lower_boll_band(&bars, now, ScreenerPeriod::Day);
    assert_eq!(result, Some(10.0));
}

#[test]
fn daily_bar_is_included_at_exactly_1500() {
    let bars = daily_bars_including("2026-07-21", 8.0);
    assert!(lower_boll_band(&bars, china_datetime(2026, 7, 21, 15, 0, 0), ScreenerPeriod::Day).unwrap() < 10.0);
}

#[test]
fn either_period_can_match_and_two_percent_boundary_is_inclusive() {
    let match_result = evaluate_boll_match(10.2, Some(10.0), Some(9.0));
    assert_eq!(match_result.periods, vec![ScreenerPeriod::Day]);
}
```

- [ ] **Step 4: Write tests for invalid technical inputs and deterministic ordering.**

```rust
#[test]
fn nonpositive_or_duplicate_bars_are_rejected_before_boll_calculation() {
    assert!(validate_completed_bars(&[bar("2026-07-20", 0.0)], ScreenerPeriod::Day).is_err());
    assert!(validate_completed_bars(&[bar("2026-07-20", 10.0), bar("2026-07-20", 11.0)], ScreenerPeriod::Day).is_err());
}

#[test]
fn result_order_uses_smallest_distance_then_code_then_exchange() {
    let ordered = sort_matches(vec![match_row("600001", "sh", 0.01), match_row("000001", "sz", 0.01)]);
    assert_eq!(ordered[0].code, "000001");
}

#[test]
fn both_or_one_available_period_can_match_but_neither_cannot() {
    assert_eq!(evaluate_boll_match(9.9, Some(10.0), Some(10.0)).periods, vec![ScreenerPeriod::Day, ScreenerPeriod::Week]);
    assert_eq!(evaluate_boll_match(10.1, Some(10.0), None).periods, vec![ScreenerPeriod::Day]);
    assert!(evaluate_boll_match(10.3, Some(10.0), Some(10.0)).periods.is_empty());
}

#[test]
fn week_is_completed_only_after_friday_close() {
    let bars = weekly_bars_including("2026-07-24", 8.0);
    assert_eq!(lower_boll_band(&bars, china_datetime(2026, 7, 24, 14, 59, 59), ScreenerPeriod::Week), Some(10.0));
}

#[test]
fn friday_bar_is_included_at_close_and_future_weekly_bars_are_excluded() {
    let bars = weekly_bars_including("2026-07-24", 8.0);
    assert!(lower_boll_band(&bars, china_datetime(2026, 7, 24, 15, 0, 0), ScreenerPeriod::Week).unwrap() < 10.0);
    assert_eq!(completed_screener_bars(&weekly_bars_including("2026-07-31", 7.0), china_datetime(2026, 7, 24, 15, 0, 0), ScreenerPeriod::Week).last().unwrap().date, "2026-07-24");
}
```

- [ ] **Step 5: Register the test module before compiling it.**

Add `pub mod screener;` to `src-tauri/src/services/mod.rs`. This intentionally exposes the incomplete test module without adding production behavior.

- [ ] **Step 6: Run the new module tests to verify failure.**

Run: `cargo test --manifest-path src-tauri/Cargo.toml services::screener::tests -- --nocapture`

Expected: FAIL because the test references undefined screener domain types and functions.

- [ ] **Step 7: Commit the failing tests.**

```bash
git add src-tauri/src/services/mod.rs src-tauri/src/services/screener.rs
git commit -m "test(screener): define screening rules"
```

### Task 2: Implement pure screening rules

**Files:**
- Modify: `src-tauri/src/services/screener.rs`
- Modify: `src-tauri/src/services/mod.rs`

- [ ] **Step 1: Add focused domain types with explicit string serialization.**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScreenerPeriod { Day, Week }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DividendAdjustment { Adjusted, VerifiedUnadjusted, Unverifiable }

#[derive(Debug, Clone)]
pub struct NormalizedDividend {
    pub code: String,
    pub exchange: String,
    pub ex_dividend_date: NaiveDate,
    pub source_cash_per_ten_shares: Option<f64>,
    pub ex_date_total_capital: Option<f64>,
    pub valuation_date_total_capital: Option<f64>,
    pub gross_per_current_share: f64,
    pub adjustment: DividendAdjustment,
    pub confirmed_cash: bool,
}

#[derive(Debug, Clone)]
pub struct CentralControllerEntry {
    pub original_name: String,
    pub normalized_name: String,
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CentralControllerRegistry {
    pub valuation_date: NaiveDate,
    pub entries: HashSet<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BollMatch {
    pub periods: Vec<ScreenerPeriod>,
    pub daily_lower_band: Option<f64>,
    pub weekly_lower_band: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct NormalizedKline {
    pub code: String,
    pub exchange: String,
    pub period: ScreenerPeriod,
    pub trading_date: NaiveDate,
    pub completed_at: String,
    pub close: f64,
}

fn test_bar(date: &str, close: f64) -> NormalizedKline {
    NormalizedKline {
        code: "600001".into(),
        exchange: "sh".into(),
        period: ScreenerPeriod::Day,
        trading_date: NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap(),
        completed_at: format!("{date}T15:00:00+08:00"),
        close,
    }
}

fn test_dividend(date: &str, amount: f64, adjustment: DividendAdjustment) -> NormalizedDividend {
    NormalizedDividend {
        code: "600001".into(),
        exchange: "sh".into(),
        ex_dividend_date: NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap(),
        source_cash_per_ten_shares: Some(amount * 10.0),
        ex_date_total_capital: None,
        valuation_date_total_capital: None,
        gross_per_current_share: amount,
        adjustment,
        confirmed_cash: true,
    }
}
```

- [ ] **Step 2: Implement the controller matcher using only NFKC, trim, and the four approved suffixes.**

```rust
pub fn normalize_controller_name(value: &str) -> String {
    let normalized: String = value.nfkc().collect();
    ["（集团）有限公司", "集团有限公司", "有限公司", "集团"]
        .iter()
        .find_map(|suffix| normalized.trim().strip_suffix(suffix))
        .unwrap_or(normalized.trim())
        .trim()
        .to_string()
}
```

Add `unicode-normalization = "0.1"` to `src-tauri/Cargo.toml`, store the normalized SASAC names and aliases in a `HashSet<String>`, and permit only the explicit direct-controller names `国务院` and `国务院国有资产监督管理委员会` plus exact set membership.

- [ ] **Step 3: Implement dividend aggregation and price validation.**

```rust
pub fn trailing_cash_dividend(records: &[NormalizedDividend], year: i32) -> Option<f64> {
    let matching: Vec<_> = records.iter().filter(|record| record.ex_dividend_date.year() == year).collect();
    if matching.iter().any(|record| record.confirmed_cash && record.adjustment == DividendAdjustment::Unverifiable) {
        return None;
    }
    let total: f64 = matching.into_iter()
        .filter(|record| record.confirmed_cash && record.adjustment != DividendAdjustment::Unverifiable)
        .map(|record| record.gross_per_current_share)
        .sum();
    (total.is_finite() && total > 0.0).then_some(total)
}
```

- [ ] **Step 4: Implement strict bar validation, daily cutoff, BOLL, and match evaluation.**

```rust
pub fn lower_boll_band(bars: &[NormalizedKline], now: DateTime<FixedOffset>, period: ScreenerPeriod) -> Option<f64> {
    let china_now = now.with_timezone(&FixedOffset::east_opt(8 * 3600).unwrap());
    let completed = completed_screener_bars(bars, china_now, period);
    validate_completed_bars(&completed, period).ok()?;
    let closes: Vec<f64> = completed.iter().rev().take(20).map(|bar| bar.close).collect();
    if closes.len() != 20 { return None; }
    let mean = closes.iter().sum::<f64>() / 20.0;
    let deviation = (closes.iter().map(|close| (close - mean).powi(2)).sum::<f64>() / 20.0).sqrt();
    let lower = mean - 2.0 * deviation;
    (lower.is_finite() && lower > 0.0).then_some(lower)
}
```

Convert every input time to Asia/Shanghai before evaluating cutoffs. Only use a same-date daily bar after `15:00:00` Asia/Shanghai and exclude all daily bars dated after the China valuation date. For weekly bars, use the provider's `completed_at` timestamp as the source of truth: include only bars whose completion timestamp is not later than the current China time and whose trading date is not in the future. This handles holidays where the exchange closes before Friday without hard-coding a weekday. Reject invalid, unordered, duplicate-date, or nonpositive closes before calculating.

- [ ] **Step 5: Run the module tests to verify they pass.**

Run: `cargo test --manifest-path src-tauri/Cargo.toml services::screener::tests -- --nocapture`

Expected: PASS.

- [ ] **Step 6: Commit the domain service.**

```bash
git add src-tauri/Cargo.toml src-tauri/src/services/mod.rs src-tauri/src/services/screener.rs
git commit -m "feat(screener): add deterministic screening rules"
```

### Task 3: Test and implement source records and Eastmoney universe parsing

**Files:**
- Create: `src-tauri/src/services/screener_source.rs`
- Create: `src-tauri/src/services/screener_source_eastmoney_universe.rs`
- Modify: `src-tauri/src/services/mod.rs`
- Test: `src-tauri/src/services/screener_source_eastmoney_universe.rs`

- [ ] **Step 1: Write parser tests for pagination, identity, eligibility, and controller classification.**

```rust
#[test]
fn all_declared_pages_are_required_and_totals_must_stay_consistent() {
    assert!(collect_exchange_pages("sh", vec![page(2, 1, sh_row("600001"))]).is_err());
    assert!(collect_exchange_pages("sh", vec![page(2, 1, sh_row("600001")), page(3, 2, sh_row("600002"))]).is_err());
}

#[test]
fn equal_duplicates_are_accepted_but_conflicting_identifiers_are_rejected() {
    assert_eq!(collect_exchange_pages("sh", vec![page(1, 1, vec![sh_row("600001"), sh_row("600001")])]).unwrap().len(), 1);
    assert!(collect_exchange_pages("sh", vec![page(1, 1, sh_row_with_secid("600001", "1.600001")), page(1, 1, sh_row_with_secid("600001", "1.999999"))]).is_err());
}

#[test]
fn universe_requires_ordinary_equity_in_each_exchange_and_exact_controller_classification() {
    assert!(validate_complete_universe(vec![sh_row("600001")], vec![sz_row("000001")], vec![]).is_err());
    assert_eq!(classify_controller("中国移动通信集团有限公司", &registry()), ControllerClassification::CentralSoe);
    assert_eq!(classify_controller("某市国资委", &registry()), ControllerClassification::LocalSoe);
}
```

- [ ] **Step 2: Register `pub mod screener_source;` and `pub mod screener_source_eastmoney_universe;`, then run the test to verify failure.**

Run: `cargo test --manifest-path src-tauri/Cargo.toml services::screener_source_eastmoney_universe::tests -- --nocapture`

Expected: FAIL because the source types and page parser functions are undefined.

- [ ] **Step 3: Implement source-agnostic records and the refresh-facing trait in `screener_source.rs`.**

```rust
pub enum ControllerClassification { CentralSoe, LocalSoe, NonSoe, Unknown }

pub struct ScreenerUniverseRecord {
    pub code: String,
    pub exchange: String,
    pub name: String,
    pub eastmoney_secid: String,
    pub ordinary_equity: bool,
    pub market_cap_cny: f64,
    pub actual_controller: String,
    pub controller_classification: ControllerClassification,
}

#[derive(Debug, Clone)]
pub struct RawDividend {
    pub ex_dividend_date: Option<NaiveDate>,
    pub cash_per_ten_shares: Option<f64>,
    pub distribution_kind: DividendKind,
    pub confirmed: bool,
    pub per_share_basis: PerShareBasis,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DividendKind { Cash, SpecialCash, NonCash }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerShareBasis { ExDate, Current, Unknown }

#[derive(Debug, Clone)]
pub struct NormalizedQuote {
    pub code: String,
    pub exchange: String,
    pub price: f64,
    pub observed_at: String,
}

#[derive(Debug, Clone)]
pub struct NormalizedKline {
    pub code: String,
    pub exchange: String,
    pub period: ScreenerPeriod,
    pub trading_date: NaiveDate,
    pub completed_at: String,
    pub close: f64,
}

pub trait ScreenerSource {
    fn fetch_controller_registry(&self, valuation_date: NaiveDate) -> Result<CentralControllerRegistrySnapshot, String>;
    fn fetch_universe(&self, valuation_date: NaiveDate) -> Result<Vec<ScreenerUniverseRecord>, String>;
    fn fetch_dividends(&self, code: &str, exchange: &str, year: i32, valuation_date: NaiveDate) -> Result<Vec<NormalizedDividend>, String>;
    fn fetch_quote(&self, code: &str, exchange: &str) -> Result<NormalizedQuote, String>;
    fn fetch_klines(&self, code: &str, exchange: &str, period: ScreenerPeriod) -> Result<Vec<NormalizedKline>, String>;
}
```

`NormalizedKline` carries the immutable code/exchange/period identity, ISO China trading date, and its UTC RFC 3339 completion timestamp. The source parser must return strict ascending, duplicate-free `completed_at` values and positive finite unadjusted CNY closes; the domain checks that all records match the request before BOLL selection. `NormalizedQuote` carries the same immutable code/exchange identity, a positive finite CNY price, and an RFC 3339 observation timestamp. `NormalizedDividend` carries code, exchange, source cash-per-10 value, ex-date capital, valuation-date capital, basis status, and normalized gross per-current-share value. Quote, run, and snapshot timestamps use UTC RFC 3339. `fetch_controller_registry` returns full rows and provenance so `screener_refresh` can persist/reuse them before requesting the classified universe. The `valuation_date` parameter ensures the market transport reads both ex-date and valuation-date total share capital before it returns a `NormalizedDividend`. `EastmoneyScreenerSource` owns a `SasacDirectoryClient` and applies the persisted or fresh registry to controller profiles without exposing provider JSON to commands.

- [ ] **Step 4: Implement Eastmoney universe/page/profile parsing in `screener_source_eastmoney_universe.rs`.**

Define `EASTMONEY_LIST_URL` as `https://82.push2.eastmoney.com/api/qt/clist/get` and `EASTMONEY_DATACENTER_URL` as `https://datacenter-web.eastmoney.com/api/data/v1/get`. Universe requests use `pn={page}`, `pz=500`, `np=1`, and require `data.total`, `data.diff[].f12` (code), `f14` (name), `f20` (CNY total market cap), and `f13` (market); calculate `page_count = ceil(total / 500)` and fetch every page from `1..=page_count`. Use `fs=m:1+t:2,m:1+t:23` for Shanghai, `fs=m:0+t:6,m:0+t:80` for Shenzhen, and `fs=m:0+t:81+s:2048` for Beijing; map security IDs as `sh => 1.{code}`, `sz => 0.{code}`, `bj => 0.{code}`. Establish `ordinary_equity` from the exchange/code whitelist: Shanghai `600|601|603|605|688`, Shenzhen `000|001|002|003|300|301`, and Beijing first digit `4|8|9`; reject all other codes before controller fetching. Deduplicate by `(code, exchange)`: equal duplicate security IDs are accepted, different IDs are an error. Reject missing pages, inconsistent page totals, conflicting security IDs, nonfinite/negative capitalization, or absent ordinary equity.

The controller-profile request uses `reportName=RPT_F10_EH_EQUITY`, filter `SECURITY_CODE='{code}'`, and requires `ACTUAL_CONTROLLER` plus `CONTROLLER_TYPE`. Normalize type values by trim and NFKC. Map `地方国有企业`, `地方国资委`, and `地方国企` to `LocalSoe`; map an exact SASAC/direct-name match to `CentralSoe`; map nonempty remaining controller names to `NonSoe`; and map missing controller/type fields to `Unknown`. Add fixture assertions for all four classifications. Never use substring matching.

- [ ] **Step 5: Run universe tests to verify they pass.**

Run: `cargo test --manifest-path src-tauri/Cargo.toml services::screener_source_eastmoney_universe::tests -- --nocapture`

Expected: PASS.

- [ ] **Step 6: Commit the universe source.**

```bash
git add src-tauri/src/services/mod.rs src-tauri/src/services/screener_source.rs src-tauri/src/services/screener_source_eastmoney_universe.rs
git commit -m "feat(screener): normalize Eastmoney universe sources"
```

### Task 4: Test and implement Eastmoney market, dividend, and capital normalization

**Files:**
- Create: `src-tauri/src/services/screener_source_eastmoney_market.rs`
- Modify: `src-tauri/src/services/mod.rs`
- Test: `src-tauri/src/services/screener_source_eastmoney_market.rs`

- [ ] **Step 1: Write parser tests for quote/K-line validation and dividend adjustment outcomes.**

```rust
#[test]
fn quote_and_kline_parser_reject_nonpositive_values_and_invalid_timestamps() {
    assert!(parse_quote("sh", invalid_price_quote()).is_err());
    assert!(parse_klines("sh", ScreenerPeriod::Week, invalid_timestamp_kline_payload()).is_err());
}

#[test]
fn cash_per_ten_and_share_capital_normalize_to_current_share_basis() {
    let record = normalize_dividend(raw_dividend("10", "2025-06-01", "cash", "confirmed"), Some(1_000.0), Some(2_000.0)).unwrap();
    assert_eq!(record.gross_per_current_share, 0.5);
    assert_eq!(record.adjustment, DividendAdjustment::Adjusted);
}

#[test]
fn missing_capital_is_unverifiable_unless_provider_declares_current_share_basis() {
    assert_eq!(normalize_dividend(raw_dividend("10", "2025-06-01", "cash", "confirmed"), None, None).unwrap().adjustment, DividendAdjustment::Unverifiable);
    assert_eq!(normalize_dividend(current_basis_dividend("1.0"), None, None).unwrap().adjustment, DividendAdjustment::VerifiedUnadjusted);
}
```

- [ ] **Step 2: Register `pub mod screener_source_eastmoney_market;` and run the test to verify failure.**

Run: `cargo test --manifest-path src-tauri/Cargo.toml services::screener_source_eastmoney_market::tests -- --nocapture`

Expected: FAIL because the market parser is not implemented.

- [ ] **Step 3: Implement the focused market-data parser.**

Define `EASTMONEY_QUOTE_URL` as `https://push2.eastmoney.com/api/qt/stock/get` and `EASTMONEY_KLINE_URL` as `https://push2his.eastmoney.com/api/qt/stock/kline/get`. Request quote `f43,f124`; convert positive `f43` cents to CNY and Unix `f124` seconds to a UTC RFC 3339 timestamp. Request raw K-lines with `fqt=0`, `klt=101` for daily and `klt=102` for weekly; require `f51` ISO date and finite positive `f52` close, construct the UTC completed-at timestamp for the corresponding China session close, and return strict ascending `NormalizedKline` records.

Define the dividend report as `RPT_F10_DIVIDEND`, requiring `EX_DIVIDEND_DATE`, `CASH_DIVIDEND_RATIO`, `DIVIDEND_TYPE`, `PLAN_STATUS`, and `PER_SHARE_BASIS`; define the capital report as `RPT_F10_CAPITAL`, requiring `CHANGE_DATE` and `TOTAL_SHARE_CAPITAL`. Map `DIVIDEND_TYPE=cash` to `Cash`, `DIVIDEND_TYPE=special_cash` to `SpecialCash`, and every other value to `NonCash`; both cash variants are eligible only when `PLAN_STATUS=confirmed`. Convert `CASH_DIVIDEND_RATIO` from per-10-share to per-ex-date-share, use both total-capital values for `Adjusted`, accept `PER_SHARE_BASIS=current` for `VerifiedUnadjusted`, and produce `Unverifiable` otherwise. Non-cash, unconfirmed, malformed, or date-less rows remain noneligible records. Add fixtures covering ordinary cash, special cash, non-cash, unconfirmed, and missing-date cases.

- [ ] **Step 4: Run market-source tests and existing quote tests.**

Run: `cargo test --manifest-path src-tauri/Cargo.toml services::screener_source_eastmoney_market::tests -- --nocapture`

Run: `cargo test --manifest-path src-tauri/Cargo.toml services::quote::tests -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Commit the market source.**

```bash
git add src-tauri/src/services/mod.rs src-tauri/src/services/screener_source_eastmoney_market.rs
git commit -m "feat(screener): normalize Eastmoney market data"
```

### Task 5: Test and implement SASAC registry parsing

**Files:**
- Create: `src-tauri/src/services/screener_source_sasac.rs`
- Create: `src-tauri/resources/sasac_controller_aliases.json`
- Modify: `src-tauri/src/services/mod.rs`
- Modify: `src-tauri/Cargo.toml`
- Test: `src-tauri/src/services/screener_source_sasac.rs`

- [ ] **Step 1: Write parser tests for an official-directory fixture containing legal names and historical aliases.**

```rust
#[test]
fn sasac_directory_preserves_source_metadata_and_aliases() {
    let html = r#"<article><a>中国移动通信集团有限公司</a></article>"#;
    let snapshot = parse_sasac_directory("2026-07-21", html).unwrap();
    assert_eq!(snapshot.source_url, SASAC_DIRECTORY_URL);
    assert!(snapshot.entries.iter().any(|entry| entry.normalized_name == "中国移动通信"));
}

#[test]
fn versioned_alias_registry_requires_official_provenance() {
    let aliases = parse_alias_registry(r#"[{"legal_name":"中国移动通信集团有限公司","alias":"中国移动","source_url":"https://www.sasac.gov.cn/example","announcement_date":"2020-01-01"}]"#).unwrap();
    assert_eq!(aliases[0].alias, "中国移动");
    assert!(parse_alias_registry(r#"[{"legal_name":"中国移动通信集团有限公司","alias":"中国移动"}]"#).is_err());
}

#[test]
fn malformed_or_empty_directory_is_not_a_valid_registry() {
    assert!(parse_sasac_directory("2026-07-21", "<html></html>").is_err());
}
```

- [ ] **Step 2: Register `pub mod screener_source_sasac;` and run the test to verify failure.**

Run: `cargo test --manifest-path src-tauri/Cargo.toml services::screener_source_sasac::tests -- --nocapture`

Expected: FAIL because the SASAC parser is not implemented.

- [ ] **Step 3: Add the parser dependency and implement the isolated SASAC directory client.**

Add `scraper = "0.20"` to `src-tauri/Cargo.toml`. Define `SASAC_DIRECTORY_URL` as `https://www.sasac.gov.cn/n2588035/n2641579/n2641645/index.html`. Fetch it with the existing user-agent/referer request policy. Use `scraper::Html` and `Selector::parse("article a")` to extract legal-name text from the official directory article body; reject an empty result or duplicate normalized legal names and preserve original plus normalized names. Load `sasac_controller_aliases.json` through `include_str!`; each JSON row must contain `legal_name`, `alias`, `source_url`, and `announcement_date`, and must normalize to a legal name from the official directory. Merge those aliases into the snapshot only after URL and ISO-date validation. Do not infer or fuzzy-match aliases, and do not scrape unrelated profile pages.

- [ ] **Step 4: Run the SASAC tests to verify they pass.**

Run: `cargo test --manifest-path src-tauri/Cargo.toml services::screener_source_sasac::tests -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Commit the registry client.**

```bash
git add src-tauri/Cargo.toml src-tauri/resources/sasac_controller_aliases.json src-tauri/src/services/mod.rs src-tauri/src/services/screener_source_sasac.rs
git commit -m "feat(screener): load central enterprise registry"
```

### Task 6: Compose the concrete screener source

**Files:**
- Modify: `src-tauri/src/services/screener_source.rs`
- Modify: `src-tauri/src/services/screener_source_eastmoney_universe.rs`
- Modify: `src-tauri/src/services/screener_source_eastmoney_market.rs`
- Modify: `src-tauri/src/services/screener_source_sasac.rs`
- Test: `src-tauri/src/services/screener_source.rs`

- [ ] **Step 1: Write a composition test using fake HTTP getters.**

```rust
#[test]
fn concrete_source_combines_registry_universe_profile_and_market_clients() {
    let source = EastmoneyScreenerSource::with_getters(fake_eastmoney_getter(), fake_sasac_getter());
    let records = source.fetch_universe(naive_date("2026-07-21")).unwrap();
    assert_eq!(records[0].controller_classification, ControllerClassification::CentralSoe);
}
```

- [ ] **Step 2: Run the composition test to verify failure.**

Run: `cargo test --manifest-path src-tauri/Cargo.toml services::screener_source::tests::concrete_source_combines_registry_universe_profile_and_market_clients -- --nocapture`

Expected: FAIL because `EastmoneyScreenerSource` is not composed.

- [ ] **Step 3: Implement `EastmoneyScreenerSource`.**

Construct it from `EastmoneyUniverseClient`, `EastmoneyMarketClient`, and `SasacDirectoryClient`. Its `fetch_universe(valuation_date)` loads and validates the official directory plus the embedded alias registry once, fetches all three exchange lists, retrieves every candidate controller profile under bounded concurrency, and applies `classify_controller`. Its remaining trait methods delegate to the focused market client. Keep HTTP getter injection available only under `#[cfg(test)]`.

- [ ] **Step 4: Run composition and all Chunk 1 Rust tests.**

Run: `cargo test --manifest-path src-tauri/Cargo.toml screener -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Commit the composed source.**

```bash
git add src-tauri/src/services/screener_source.rs src-tauri/src/services/screener_source_eastmoney_universe.rs src-tauri/src/services/screener_source_eastmoney_market.rs src-tauri/src/services/screener_source_sasac.rs
git commit -m "feat(screener): compose public data source"
```

## Chunk 2: Persistence, Refresh Orchestration, And Tauri Commands

### Task 7: Add failing SQLite migration and lifecycle tests

**Files:**
- Modify: `src-tauri/src/db.rs`
- Modify: `src-tauri/src/commands/data.rs`
- Test: `src-tauri/src/db.rs`
- Test: `src-tauri/src/commands/data.rs`

- [ ] **Step 1: Write failing schema tests for all derived screener tables and indexes.**

```rust
#[test]
fn migration_creates_screener_cache_and_result_tables() {
    let conn = test_connection_after_migration();
    for table in ["screener_fundamentals", "screener_controller_registry", "screener_quotes", "screener_runs", "screener_results", "screener_state"] {
        assert!(table_exists(&conn, table));
    }
    assert!(index_exists(&conn, "idx_screener_results_run"));
}

#[test]
fn import_clears_derived_screener_rows_and_increments_generation() {
    let conn = test_connection_after_migration();
    seed_screener_rows(&conn);
    let generation_before = screener_generation(&conn);
    import_payload_to_conn(&conn, empty_export_payload()).unwrap();
    assert_eq!(screener_row_count(&conn), 0);
    assert_eq!(screener_generation(&conn), generation_before + 1);
}

#[test]
fn result_rows_require_a_run_and_import_rolls_back_all_screener_changes_on_error() {
    let conn = test_connection_after_migration();
    assert!(insert_result_without_run(&conn).is_err());
    seed_screener_rows(&conn);
    assert!(import_payload_to_conn(&conn, invalid_export_payload()).is_err());
    assert_eq!(screener_row_count(&conn), 1);
}
```

- [ ] **Step 2: Run migration and import tests to verify failure.**

Run: `cargo test --manifest-path src-tauri/Cargo.toml migration_creates_screener_cache_and_result_tables -- --nocapture`

Run: `cargo test --manifest-path src-tauri/Cargo.toml import_clears_derived_screener_rows_and_increments_generation -- --nocapture`

Expected: FAIL because the derived tables and generation handling do not exist.

- [ ] **Step 3: Add idempotent derived-cache schema and indexes.**

Create these tables in both `init_db` and `migrate`:

```sql
CREATE TABLE IF NOT EXISTS screener_fundamentals (
  code TEXT NOT NULL, exchange TEXT NOT NULL, valuation_date TEXT NOT NULL,
  dividend_year INTEGER NOT NULL, name TEXT NOT NULL, market_cap_cny REAL NOT NULL,
  actual_controller TEXT NOT NULL, controller_classification TEXT NOT NULL,
  cash_dividend_per_share REAL NOT NULL, source_metadata TEXT NOT NULL,
  observed_at TEXT NOT NULL,
  PRIMARY KEY (code, exchange, valuation_date, dividend_year)
);
CREATE TABLE IF NOT EXISTS screener_controller_registry (
  valuation_date TEXT NOT NULL, legal_name TEXT NOT NULL, normalized_name TEXT NOT NULL,
  aliases_json TEXT NOT NULL, source_url TEXT NOT NULL, source_date TEXT NOT NULL,
  PRIMARY KEY (valuation_date, normalized_name)
);
CREATE TABLE IF NOT EXISTS screener_quotes (
  code TEXT NOT NULL, exchange TEXT NOT NULL, valuation_date TEXT NOT NULL,
  price REAL NOT NULL, observed_at TEXT NOT NULL,
  PRIMARY KEY (code, exchange, valuation_date)
);
CREATE TABLE IF NOT EXISTS screener_runs (
  id INTEGER PRIMARY KEY AUTOINCREMENT, status TEXT NOT NULL, started_at TEXT NOT NULL,
  completed_at TEXT NOT NULL, candidate_count INTEGER NOT NULL, match_count INTEGER NOT NULL,
  skipped_count INTEGER NOT NULL, failure_code TEXT, failure_message TEXT
);
CREATE TABLE IF NOT EXISTS screener_results (
  run_id INTEGER NOT NULL, code TEXT NOT NULL, exchange TEXT NOT NULL, name TEXT NOT NULL,
  market_cap_cny REAL NOT NULL, cash_dividend_per_share REAL NOT NULL, dividend_yield REAL NOT NULL,
  current_price REAL NOT NULL, price_observed_at TEXT NOT NULL,
  daily_lower_band REAL, daily_distance REAL, daily_kline_completed_at TEXT,
  weekly_lower_band REAL, weekly_distance REAL, weekly_kline_completed_at TEXT,
  matched_periods_json TEXT NOT NULL, source_metadata TEXT NOT NULL, fundamental_observed_at TEXT NOT NULL,
  PRIMARY KEY (run_id, code, exchange), FOREIGN KEY (run_id) REFERENCES screener_runs(id)
);
CREATE TABLE IF NOT EXISTS screener_state (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE INDEX IF NOT EXISTS idx_screener_results_run ON screener_results(run_id);
```

Seed `screener_state.generation` with `0`. All values are derived cache data, so do not add them to `ExportPayload`.

- [ ] **Step 4: Make imports invalidate derived data atomically.**

At the beginning of the existing import transaction, delete rows from `screener_fundamentals`, `screener_controller_registry`, `screener_quotes`, `screener_results`, and `screener_runs`, but never delete `screener_state`. Increment `screener_state.generation` with `UPDATE ... SET value = CAST(value AS INTEGER) + 1 WHERE key = 'generation'`, then insert `generation = 1` only when the row does not exist. Keep existing export JSON backward compatible by leaving `ExportPayload` unchanged.

- [ ] **Step 5: Implement and test bounded retention.**

Add one repository `prune_derived_screener_data` helper that retains only the newest seven distinct `valuation_date` values in `screener_fundamentals`, `screener_controller_registry`, and `screener_quotes`; it retains the newest 30 `screener_runs` of every status, deletes child `screener_results` before their parent runs, and removes no run separately elsewhere. Test that an eighth valuation date and 31st run remove the oldest rows, while a recent failed attempt remains until it falls outside the 30-run window.

- [ ] **Step 6: Run the migration/import tests to verify they pass.**

Run: `cargo test --manifest-path src-tauri/Cargo.toml db::tests commands::data::tests -- --nocapture`

Expected: PASS.

- [ ] **Step 7: Commit persistence migration.**

```bash
git add src-tauri/src/db.rs src-tauri/src/commands/data.rs
git commit -m "feat(screener): persist derived screening data"
```

### Task 8: Add screener command models and persistence helpers

**Files:**
- Modify: `src-tauri/src/models.rs`
- Create: `src-tauri/src/commands/screener_repository.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Test: `src-tauri/src/commands/screener_repository.rs`

- [ ] **Step 1: Write failing command-helper tests for atomic result replacement and stale dashboards.**

```rust
#[test]
fn completed_partial_run_replaces_displayed_results_atomically() {
    let conn = setup_screener_schema();
    persist_completed_run(&conn, partial_run("2026-07-21T01:00:00Z"), vec![result("600001")]).unwrap();
    let dashboard = read_screener_dashboard(&conn).unwrap();
    assert_eq!(dashboard.displayed_run.unwrap().status, "partial");
    assert_eq!(dashboard.results.len(), 1);
}

#[test]
fn failed_attempt_retains_last_completed_results_and_marks_dashboard_stale() {
    let conn = setup_screener_schema();
    persist_completed_run(&conn, success_run("2026-07-21T01:00:00Z"), vec![result("600001")]).unwrap();
    persist_failed_attempt(&conn, "provider_unavailable", "timeout").unwrap();
    let dashboard = read_screener_dashboard(&conn).unwrap();
    assert!(dashboard.is_stale);
    assert_eq!(dashboard.results[0].code, "600001");
}
```

- [ ] **Step 2: Run the command-helper tests to verify failure.**

Run: `cargo test --manifest-path src-tauri/Cargo.toml commands::screener_repository::tests -- --nocapture`

Expected: FAIL because the command module and dashboard DTOs do not exist.

- [ ] **Step 3: Define shared Serde DTOs in `models.rs`.**

Add `ScreenerRun`, `ScreenerFailureSummary`, `ScreenerResult`, `ScreenerDashboard`, `ScreenerRefreshSchedule`, and `ScreenerWatchlistAddResult`. Use `Option<f64>` and `Option<String>` for unavailable period fields, `Vec<String>` for `matched_periods`, and preserve the exact field names in the approved dashboard contract. Add `#[serde(rename_all = "snake_case")]` to status/failure enums.

- [ ] **Step 4: Implement only database mappers and atomic writers.**

In `commands/screener_repository.rs`, keep SQL row mapping and persistence helpers separate from network refresh code:

```rust
fn persist_completed_run(conn: &Connection, run: PendingScreenerRun, results: Vec<ScreenerResult>) -> Result<ScreenerDashboard, String> {
    let tx = conn.unchecked_transaction().map_err(|error| error.to_string())?;
    // insert one run, insert only its matching result rows, prune old completed runs, commit
    tx.commit().map_err(|error| error.to_string())?;
    read_screener_dashboard(conn)
}
```

`persist_failed_attempt` writes a failure run with no result rows and never deletes prior successful/partial rows. `read_screener_dashboard` selects result rows only for the newest `success` or `partial` run and attaches the most recent later failure as `latest_failed_attempt`. Both writers call `prune_derived_screener_data`; result rows carry the serialized source metadata needed after their seven-day fundamental cache is pruned.

- [ ] **Step 5: Run command-helper tests to verify they pass.**

Run: `cargo test --manifest-path src-tauri/Cargo.toml commands::screener_repository::tests -- --nocapture`

Expected: PASS.

- [ ] **Step 6: Commit DTOs and persistence helpers.**

```bash
git add src-tauri/src/models.rs src-tauri/src/commands/mod.rs src-tauri/src/commands/screener_repository.rs
git commit -m "feat(screener): add dashboard persistence commands"
```

### Task 9: Implement refresh orchestration, scheduling, and one-way watchlist action

**Files:**
- Create: `src-tauri/src/commands/screener_refresh.rs`
- Create: `src-tauri/src/services/screener_calendar.rs`
- Create: `src-tauri/resources/china_market_holidays.json`
- Modify: `src-tauri/src/commands/screener.rs`
- Modify: `src-tauri/src/commands/screener_repository.rs`
- Modify: `src-tauri/src/commands/quant.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/services/quote.rs`
- Test: `src-tauri/src/commands/screener_refresh.rs`
- Test: `src-tauri/src/services/screener_calendar.rs`
- Test: `src-tauri/src/services/quote.rs`

- [ ] **Step 1: Write failing tests for generation, per-stock partial data, schedule boundaries, and idempotent watchlist add.**

```rust
#[tokio::test]
async fn refresh_discards_network_data_when_import_changes_generation() {
    let source = BlockingFakeSource::new();
    let task = tokio::spawn(refresh_with_source(state.clone(), source.clone()));
    source.wait_until_fetching();
    increment_screener_generation(&state.db.lock().unwrap()).unwrap();
    source.release();
    assert!(task.await.unwrap().displayed_run.is_none());
}

#[tokio::test]
async fn simultaneous_refresh_callers_join_one_network_run_and_cleanup_after_completion() {
    let source = CountingFakeSource::new();
    let first = tokio::spawn(refresh_with_source(state.clone(), source.clone()));
    let second = tokio::spawn(refresh_with_source(state.clone(), source.clone()));
    let _ = tokio::try_join!(first, second).unwrap();
    assert_eq!(source.universe_calls(), 1);
    assert!(!state.screener_refresh.is_running());
}

#[test]
fn schedule_returns_next_opening_during_break_weekend_holiday_and_after_close() {
    assert_eq!(schedule_at(china_time(2026, 7, 21, 11, 31)).next_refresh_at, "2026-07-21T05:00:00Z");
    assert_eq!(schedule_at(china_time(2026, 7, 25, 10, 0)).next_refresh_at, "2026-07-27T01:30:00Z");
}

#[test]
fn adding_existing_screener_result_to_cn_watchlist_is_successful_noop() {
    let result = add_screener_result_to_watchlist(&conn, "600001", "测试").unwrap();
    assert!(result.already_present);
}
```

- [ ] **Step 2: Run orchestration tests to verify failure.**

Run: `cargo test --manifest-path src-tauri/Cargo.toml commands::screener::tests -- --nocapture`

Expected: FAIL because refresh, schedule, and idempotent-add behavior are incomplete.

- [ ] **Step 3: Add `ScreenerRefreshState` to `AppState` and use it as the single-flight guard.**

Add a shared `Mutex<Option<Shared<BoxFuture<Result<ScreenerDashboard, String>>>>>` or an equivalent explicit running-state object initialized in `lib.rs`. `refresh_screener` joins an existing task rather than starting another, and clears the in-flight slot on both success and error. Read the database generation inside a short lock scope, release `Mutex<Connection>` before every network await, then reacquire it only for a generation-checked write. Capture the generation before source fetching; check it before each cache write and before the final result transaction. If it changed, discard fetched rows and return the post-import dashboard.

- [ ] **Step 4: Write failing refresh lifecycle tests.**

Cover fundamental and quote cache hits, failed fetch non-caching, independent successful snapshot persistence when a later run fails, semaphore maximum of four candidate market tasks, full-universe failure retaining stale rows, `candidate_count` after market/central-SOE filtering, `skipped_count` for only request/normalization failures, and off-hours reuse of only a same-valuation-date cached quote.

- [ ] **Step 5: Orchestrate the staged refresh with bounded concurrency.**

Load/refresh the valuation-date SASAC registry, universe, fundamentals, dividends, raw quotes, and day/week K-lines through `ScreenerSource`. Reuse only successful `(code, exchange, valuation_date, dividend_year)` fundamentals and same-date quotes. Missing cache entries retry in the same day. Filter central SOE and market cap before dividend/K-line work; aggregate dividends and require 5%; then fetch day/week bars with a semaphore limit of four. Count only request/normalization failures as skipped. Persist successful snapshots independently, collect fully evaluated candidates, and atomically persist one `success` or `partial` result run. A full universe failure writes a failed attempt and returns a stale dashboard; database failures return the Tauri error.

- [ ] **Step 6: Implement and test the backend schedule command.**

Add `ScreenerCalendar` in `services/screener_calendar.rs`, loading `china_market_holidays.json` with `{ "version": "2026", "closed_dates": ["2026-10-01"] }`. It exposes `schedule_at(DateTime<FixedOffset>) -> ScreenerRefreshSchedule`. Test opening at 09:30, 15-minute alignment, 11:31 midday break, 13:00 afternoon opening, 15:01 close, weekend, and a fixture holiday. It returns `should_refresh_now = true` only in a live session and `next_refresh_at` as UTC RFC 3339.

- [ ] **Step 7: Make screener watchlist insertion idempotent and preserve existing quant behavior.**

Add `add_screener_result_to_watchlist` that maps every screener exchange to `market = "cn"`, inserts through `INSERT OR IGNORE`, and returns `{ added, already_present }`. Test `sh => 1.{code}`, `sz => 0.{code}`, and `bj => 0.{code}`, then test that an idempotent add preserves an existing watchlist row's name and enabled state. Do not change `add_quant_watchlist`, which continues to report duplicate entry errors. Extend `stock_secid` into an exchange-aware helper used by screener code without altering existing `cn` requests.

The Tauri handler signature is `pub fn add_screener_result_to_watchlist(state: State<AppState>, code: String, exchange: String, name: String) -> Result<ScreenerWatchlistAddResult, String>`. It rejects exchanges other than `sh`, `sz`, or `bj`; this exact named-argument signature is the contract used by the TypeScript wrapper.

- [ ] **Step 8: Register the Tauri commands and run backend verification.**

Register `get_screener_dashboard`, `refresh_screener`, `get_screener_refresh_schedule`, and `add_screener_result_to_watchlist` in `lib.rs`.

Run: `cargo test --manifest-path src-tauri/Cargo.toml screener -- --nocapture`

Run: `cargo test --manifest-path src-tauri/Cargo.toml services::quote::tests -- --nocapture`

Expected: PASS.

- [ ] **Step 9: Commit orchestration and commands.**

```bash
git add src-tauri/resources/china_market_holidays.json src-tauri/src/commands/quant.rs src-tauri/src/commands/screener.rs src-tauri/src/commands/screener_refresh.rs src-tauri/src/commands/screener_repository.rs src-tauri/src/lib.rs src-tauri/src/services/screener_calendar.rs src-tauri/src/services/quote.rs
git commit -m "feat(screener): refresh and expose screening results"
```

## Chunk 3: Frontend API, Screener Page, And End-To-End Verification

### Task 10: Add frontend types, API wrappers, and pure view tests

**Files:**
- Modify: `src/types/index.ts`
- Modify: `src/api/index.ts`
- Create: `src/pages/screenerView.ts`
- Create: `src/pages/screenerView.test.ts`

- [ ] **Step 1: Write failing view-helper tests.**

```ts
import { describe, expect, it } from "vitest";
import { defaultScreenerSort, screenerState } from "./screenerView";

it("orders by the smallest available lower-band distance then code and exchange", () => {
  expect(defaultScreenerSort([row("600001", "sh", 0.01, null), row("000001", "sz", 0.01, null)]).map((item) => item.code))
    .toEqual(["000001", "600001"]);
});

it("distinguishes first use, no match, stale, and failed states", () => {
  expect(screenerState(emptyDashboard())).toBe("first-use");
  expect(screenerState(staleDashboard())).toBe("stale");
});
```

- [ ] **Step 2: Run view tests to verify failure.**

Run: `npm test -- src/pages/screenerView.test.ts`

Expected: FAIL because the helper module does not exist.

- [ ] **Step 3: Define TypeScript contracts that mirror the Tauri DTOs.**

Add `ScreenerRunStatus`, `ScreenerFailureCode`, `ScreenerResult`, `ScreenerRun`, `ScreenerDashboard`, `ScreenerRefreshSchedule`, and `ScreenerWatchlistAddResult` to `src/types/index.ts`. Keep Rust snake_case fields and nullable daily/weekly values exactly intact. Add `api.getScreenerDashboard`, `api.refreshScreener`, `api.getScreenerRefreshSchedule`, and `api.addScreenerResultToWatchlist(code: string, exchange: "sh" | "sz" | "bj", name: string)`, invoking `add_screener_result_to_watchlist` with `{ code, exchange, name }`. Add an API-unit test that mocks `invoke` and asserts the command name and exact three named arguments.

- [ ] **Step 4: Implement pure display helpers.**

`defaultScreenerSort` takes the smaller non-null daily/weekly distance, then code, then exchange. `screenerState` returns `first-use`, `no-match`, `ready`, `stale`, or `failed` based only on the dashboard contract. Add formatters for CNY market cap in billions, dividend yield percentage, and nullable numeric values; do not make API calls in this file.

- [ ] **Step 5: Run view tests to verify they pass.**

Run: `npm test -- src/pages/screenerView.test.ts`

Expected: PASS.

- [ ] **Step 6: Commit frontend contracts.**

```bash
git add src/types/index.ts src/api/index.ts src/pages/screenerView.ts src/pages/screenerView.test.ts
git commit -m "feat(screener): add frontend data contracts"
```

### Task 11: Build the screener page and routing integration

**Files:**
- Create: `src/pages/Screener.tsx`
- Modify: `src/App.tsx`
- Modify: `src/styles/global.css`
- Modify: `package.json`
- Modify: `vite.config.ts`
- Create: `src/test/setup.ts`
- Create: `src/pages/Screener.test.tsx`

- [ ] **Step 1: Configure rendered-page testing and write failing workflow tests.**

Add `@testing-library/react`, `@testing-library/user-event`, and `jsdom` as dev dependencies. Configure Vitest through Vite with `environment: "jsdom"` and `setupFiles: ["./src/test/setup.ts"]`. In setup, call `afterEach(cleanup)`, `vi.clearAllMocks()`, `vi.useRealTimers()`, and define `matchMedia`/`ResizeObserver` fallbacks. Mock Tauri `listen` because the application shell subscribes during mount. Define `renderScreener` to call `window.history.pushState({}, "", "/screener")` then render `<AppThemeProvider><FinanceApp /></AppThemeProvider>`; the existing `FinanceApp` supplies its own Ant Design `App` and router. Mock `api`; use fake timers to prove one active timeout after refresh or schedule retrieval failure and zero after unmount. Cover first-use, no-match, stale cached result, failed-without-cache, refreshing/manual button state, timer rearming, and `sh`/`sz`/`bj` watchlist invocation and feedback.

- [ ] **Step 2: Run the page tests to verify failure.**

Run: `npm test -- src/pages/Screener.test.tsx`

Expected: FAIL because the page and DOM test setup do not exist.

- [ ] **Step 3: Implement the page’s initial dashboard load and explicit state branches.**

Use `PageLoader`, `PageHeader`, `EmptyPlaceholder`, `Alert`, `Card`, `Table`, `Tag`, `Tooltip`, `Button`, and `Modal` consistently with `QuantAlerts.tsx`. Load `api.getScreenerDashboard()` on mount; retain the latest returned dashboard while a refresh runs; surface Tauri command errors through `formatInvokeError`. Render first-use, no-match, stale, and failed states from `screenerState` rather than inferring from ad hoc local flags.

Use this test decision table: `displayed_run = null` with no failed attempt renders `first-use` and a manual refresh action; `displayed_run = null` with a failed attempt renders `failed` plus failure summary/action; a `success`/`partial` run with zero results renders `no-match` and its completed timestamp; a displayed run plus later failed attempt or `is_stale = true` renders `stale`, retains result rows, and shows both the completed timestamp and failure summary; a partial run renders its skipped count. Assert these states in `Screener.test.tsx` before implementing the corresponding branch.

- [ ] **Step 4: Implement page-lifetime automatic refresh scheduling.**

On mount and after every refresh attempt, call `api.getScreenerRefreshSchedule()`. Maintain a monotonically increasing schedule request generation. Before setting a new timeout, always clear the stored timer ID; only apply a response whose generation is current and whose component is still mounted. The timer callback calls the shared refresh function, then requests and arms the next schedule. The effect cleanup clears the timeout and invalidates pending generations on unmount. If `should_refresh_now` is true, request `api.refreshScreener()` once after dashboard load. On schedule retrieval or refresh failure, preserve the current dashboard, show the error, and schedule a retry through a one-minute fallback timer that is replaced by the next successful backend schedule. Manual refresh calls the same function, disables the button while active, and then re-arms the schedule. Do not start a global frontend interval.

- [ ] **Step 5: Implement the compact strategy/status area.**

Display match count, successful refresh timestamp, an explicit “刷新中”/“已刷新”/“数据过期” status tag, stale/failure summary, and manual refresh action above a read-only condition summary: central SOE, CNY 50 billion market cap, 5% trailing cash dividend yield, and 2% daily-or-weekly BOLL lower-band distance. Do not add adjustable controls because the approved strategy is fixed.

- [ ] **Step 6: Implement the dense sortable results table.**

Default `dataSource` through `defaultScreenerSort`. Include name/code, exchange, market cap, preceding-year cash dividend per share, dividend yield, current price, daily lower band/distance, weekly lower band/distance, matching periods, and timestamps. Render `Table` with `scroll={{ x: 1480 }}`. Use fixed `minWidth`/ellipsis renderers so table text does not reflow controls. Provide Ant Design sortable columns for name, market cap, dividend yield, and the default distance. Use `Tag` for `day` and `week` matches, and display unavailable values as `--`.

- [ ] **Step 7: Implement row actions with existing patterns.**

Create a zero-quantity `Holding` from the result to open `KlineChart` inside the existing-style modal. Add an icon-only `PlusOutlined` watchlist action inside `Tooltip`; call `api.addScreenerResultToWatchlist`, then show either “已加入量化关注” or “已在量化关注列表中” from the returned flags. Do not expose trading actions.

- [ ] **Step 8: Add navigation and responsive CSS.**

In `App.tsx`, add `FilterOutlined` for `/screener`, import `ScreenerPage`, and add title/subtitle metadata. In `global.css`, add `.screener-status-grid`, `.screener-rule-list`, `.screener-table`, and a mobile breakpoint that makes the status grid one column while preserving horizontal table scroll. Keep the visual system consistent with existing `stat-card`, `filter-bar`, and `quant-status-grid` styles.

- [ ] **Step 9: Run rendered-page tests, type checking, and production build.**

Run: `npm test -- src/pages/Screener.test.tsx`

Expected: PASS.

Set the jsdom viewport to `390 x 844` in a dedicated rendered test. Assert `/screener` navigation, visible manual refresh/status controls, `Table` horizontal-scroll configuration, and tooltip-labelled K-line/watchlist actions; restore the viewport after the test.

In the desktop Tauri validation at `390 x 844`, manually open the screener route, trigger refresh, horizontally scroll the table to the action column, open and close the K-line modal, and add a result to the quant watchlist. Pass only when status/action controls are visible without clipping, the modal is usable, and watchlist feedback is visible.

Run: `npm run typecheck`

Expected: PASS.

Run: `npm run build`

Expected: PASS.

- [ ] **Step 10: Commit the screener page.**

```bash
git add package.json package-lock.json src/App.tsx src/pages/Screener.tsx src/pages/Screener.test.tsx src/styles/global.css src/test/setup.ts vite.config.ts
git commit -m "feat(screener): add screening results page"
```

### Task 12: Run full verification and inspect the working application

**Files:**
- Verify: `src-tauri/src/services/screener.rs`
- Verify: `src-tauri/src/services/screener_source*.rs`
- Verify: `src-tauri/src/commands/screener*.rs`
- Verify: `src/pages/Screener.tsx`

- [ ] **Step 1: Run the complete Rust suite.**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: PASS.

- [ ] **Step 2: Run all frontend tests, type checks, and production build.**

Run: `npm test`

Expected: PASS.

Run: `npm run typecheck && npm run build`

Expected: PASS.

- [ ] **Step 3: Start the application and validate the user workflow.**

Run: `npm run tauri dev`

Expected: The desktop application launches, “选股” appears in the sidebar, first-use/no-match/stale/failed-without-cache states are visually distinct, manual refresh transitions through loading and re-arms exactly one timer, sortable columns reorder rows, K-line and watchlist actions work, and network failures preserve a stale result rather than blanking the table. Resize the Tauri window to `390 x 844` and verify navigation, visible refresh/status controls, horizontally scrollable table, and accessible row actions.

- [ ] **Step 4: Inspect the final diff and commit final verification fixes if required.**

Run: `BASE_HEAD=$(cat /var/folders/9l/4ybk4rnn3255lfv5vvnjnp340000gn/T/opencode/screener-base-head.txt) && git diff --check "$BASE_HEAD"..HEAD && git diff --check && git diff "$BASE_HEAD"..HEAD -- src-tauri/src/db.rs src-tauri/src/models.rs src-tauri/src/services src-tauri/src/commands src/App.tsx src/api/index.ts src/types/index.ts src/pages/Screener.tsx src/pages/screenerView.ts src/styles/global.css package.json vite.config.ts && git status --short > /var/folders/9l/4ybk4rnn3255lfv5vvnjnp340000gn/T/opencode/screener-status-after.txt && diff -u /var/folders/9l/4ybk4rnn3255lfv5vvnjnp340000gn/T/opencode/screener-status-before.txt /var/folders/9l/4ybk4rnn3255lfv5vvnjnp340000gn/T/opencode/screener-status-after.txt`

Expected: No whitespace errors; the diff contains only intended screener changes relative to the captured user-worktree baseline.

- [ ] **Step 5: Run formatting checks.**

Run: `cargo fmt --manifest-path src-tauri/Cargo.toml --check`

Expected: PASS.
