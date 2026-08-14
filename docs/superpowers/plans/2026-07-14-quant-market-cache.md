# Quant Market Cache Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Cache same-day historical quant K-lines and intraday-T window statistics while retaining per-refresh realtime quotes and bounded target concurrency.

**Architecture:** Add a process-local `QuantMarketCache` to `AppState`. Its keys include China trading date, market, code, K-line limits, and intraday analysis settings; only successful historical results enter it. The quant snapshot loader checks or fills this cache without holding its mutex across network awaits, and uses a semaphore to bound concurrent target fetches.

**Tech Stack:** Rust, Tokio `Mutex`/`Semaphore`, Tauri 2, rusqlite, existing Rust unit tests.

---

## File Structure

- `src-tauri/src/services/quant_cache.rs`: Cache keys, historical K-line entries, intraday-T statistics entries, and invalidation-by-key behavior. No network or Tauri dependencies.
- `src-tauri/src/services/mod.rs`: Export the cache module.
- `src-tauri/src/lib.rs`: Add the in-memory cache mutex to `AppState` and initialize it at startup.
- `src-tauri/src/commands/quant.rs`: Build complete cache keys from target settings, use cached historical data when loading snapshots, compute/cache intraday statistics, and apply bounded concurrency.

## Chunk 1: Cache Data Model

### Task 1: Add and test cache keys and successful-result storage

**Files:**
- Create: `src-tauri/src/services/quant_cache.rs`
- Modify: `src-tauri/src/services/mod.rs`

- [ ] **Step 1: Write failing cache behavior tests**

Create `quant_cache.rs` with a `#[cfg(test)]` module before implementing the cache. Add tests that require:

```rust
#[test]
fn daily_history_key_changes_on_trading_date_or_limit() { /* ... */ }

#[test]
fn intraday_stats_key_changes_when_any_analysis_setting_changes() { /* ... */ }

#[test]
fn cache_returns_only_successfully_inserted_history_and_stats() { /* ... */ }
```

Use fixture `KlineBar` values and `IntradayTWindowStats`. Assert that an uninserted/failed result is absent, a stored entry is retrievable, and a different date, daily limit, lookback, high threshold, or low threshold cannot hit that entry.

- [ ] **Step 2: Run the cache tests to verify RED**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib services::quant_cache::tests`

Expected: FAIL because `quant_cache` is not exported or the cache types do not exist.

- [ ] **Step 3: Implement the dependency-free cache module**

Define private/hashable key structs and an exported cache owner:

```rust
pub struct QuantMarketCache {
    daily_bars: HashMap<DailyHistoryKey, Vec<KlineBar>>,
    intraday: HashMap<IntradayHistoryKey, IntradayHistoryEntry>,
}

pub struct IntradayHistoryEntry {
    pub bars: Vec<KlineBar>,
    pub stats: IntradayTWindowStats,
}
```

The daily key contains `code`, `market`, China `trading_date`, and `limit`. The intraday key contains those values plus `lookback_days`, `high_min_count`, and `low_min_count`; its historical limit is derived from the lookback and therefore does not need a separate field. Expose small `get_*` and `insert_*` methods taking key construction inputs. Only callers that receive `Ok(Vec<KlineBar>)` may call `insert_*`; the cache module must not store an error variant.

Add `pub mod quant_cache;` in `services/mod.rs`.

- [ ] **Step 4: Run cache tests to verify GREEN**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib services::quant_cache::tests`

Expected: PASS with all cache-key and successful-storage tests passing.

- [ ] **Step 5: Commit the cache model**

```bash
git add src-tauri/src/services/quant_cache.rs src-tauri/src/services/mod.rs
git commit -m "feat(quant): add historical market cache"
```

Expected: one commit containing only the cache module and its tests.

## Chunk 2: Cached Snapshot Loading

### Task 2: Add cache ownership and cached historical snapshot fetches

**Files:**
- Modify: `src-tauri/src/lib.rs:16-18,37-42`
- Modify: `src-tauri/src/commands/quant.rs:22-37,1075-1144`
- Test: `src-tauri/src/commands/quant.rs` existing test module

- [ ] **Step 1: Add failing snapshot-cache tests**

Add focused tests around an extracted snapshot-loading helper that accepts injected daily/minute/quote fetch closures and a shared `Mutex<QuantMarketCache>`. Cover:

```rust
#[tokio::test]
async fn same_day_daily_history_is_fetched_once_but_quotes_are_fetched_each_refresh() { /* ... */ }

#[tokio::test]
async fn same_day_intraday_history_reuses_cached_bars_and_stats() { /* ... */ }

#[tokio::test]
async fn failed_historical_fetch_is_not_cached() { /* ... */ }
```

Use atomic counters in the injected closures. The second request must reuse successful daily/minute data yet call the quote fetch again. A failed first historical request must cause a second request on the next refresh.

- [ ] **Step 2: Run focused tests to verify RED**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib commands::quant::tests::same_day_daily_history_is_fetched_once_but_quotes_are_fetched_each_refresh -- --exact`

Expected: FAIL because the cache-aware helper is not implemented.

- [ ] **Step 3: Add cache ownership to application state**

Extend `AppState`:

```rust
pub struct AppState {
    pub db: Mutex<rusqlite::Connection>,
    pub quant_market_cache: Mutex<QuantMarketCache>,
}
```

Initialize the cache in `run()` alongside the database connection. Use `std::sync::Mutex` only for short clone/read/insert critical sections; never retain its guard across an `.await`.

- [ ] **Step 4: Implement cache-aware snapshot loading**

Extend `QuantSnapshotRequest` to carry the China trading date and required historical settings. Extract a helper that:

1. Checks the daily key and clones cached bars when present; otherwise awaits the daily fetch and inserts only successful bars.
2. For intraday-T requests, checks the intraday key and returns cached bars/statistics when present; otherwise awaits the 5-minute fetch, calculates `analyze_intraday_t_windows`, and stores both only after success.
3. Always awaits the realtime quote fetch independently of cache state.

Update `QuantMarketSnapshot` to carry optional precomputed intraday statistics. In `build_quant_dashboard_with_snapshots`, consume those cached statistics when available; retain `analyze_intraday_t_windows` as a fallback only for test-created snapshots that do not supply it.

Use `china_market_now()` once when creating requests, format its date once, and include that date in all cache lookups. Therefore, a new China date automatically misses old cache entries. Settings changes naturally miss because they change the intraday key.

- [ ] **Step 5: Run cache and regression tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib commands::quant::tests::same_day_daily_history_is_fetched_once_but_quotes_are_fetched_each_refresh -- --exact
cargo test --manifest-path src-tauri/Cargo.toml --lib commands::quant::tests::same_day_intraday_history_reuses_cached_bars_and_stats -- --exact
cargo test --manifest-path src-tauri/Cargo.toml --lib commands::quant::tests::failed_historical_fetch_is_not_cached -- --exact
cargo test --manifest-path src-tauri/Cargo.toml --lib commands::quant::tests::intraday_t_dashboard_keeps_windows_when_quote_is_stale -- --exact
```

Expected: PASS. The existing stale-quote test confirms cached/derived windows do not bypass strict realtime signal checks.

- [ ] **Step 6: Commit cached snapshot loading**

```bash
git add src-tauri/src/lib.rs src-tauri/src/commands/quant.rs
git commit -m "feat(quant): cache historical snapshot data"
```

Expected: one commit containing cache ownership, loader integration, and focused command tests.

## Chunk 3: Provider Protection And Full Verification

### Task 3: Bound target concurrency and verify the complete feature

**Files:**
- Modify: `src-tauri/src/commands/quant.rs:1124-1144`
- Test: `src-tauri/src/commands/quant.rs` existing test module

- [ ] **Step 1: Add a failing concurrency-limit test**

Add an async test for an extracted `fetch_quant_snapshots_concurrently` variant taking a limit. Use an atomic active-count and maximum-count tracker in the injected future. Queue more requests than the proposed limit and assert the maximum active count never exceeds `4`, while all snapshots are returned.

- [ ] **Step 2: Run the test to verify RED**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib commands::quant::tests::snapshot_fetches_respect_concurrency_limit -- --exact`

Expected: FAIL because the current implementation spawns every request at once.

- [ ] **Step 3: Implement bounded concurrency**

Use `tokio::sync::Semaphore` with a module constant:

```rust
const QUANT_SNAPSHOT_CONCURRENCY: usize = 4;
```

Acquire a permit inside each spawned task before invoking the fetch closure. Await all handles and preserve the existing behavior of returning successfully completed snapshots. Do not hold the database or cache mutex while waiting for a permit or network request.

- [ ] **Step 4: Run focused and full verification**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib commands::quant::tests::snapshot_fetches_respect_concurrency_limit -- --exact
cargo test --manifest-path src-tauri/Cargo.toml --lib
npm run test
npm run typecheck
npm run build
```

Expected: concurrency test and full Rust library suite pass; frontend tests, TypeScript, and production build pass.

- [ ] **Step 5: Commit the concurrency guard**

```bash
git add src-tauri/src/commands/quant.rs
git commit -m "perf(quant): bound snapshot fetch concurrency"
```

Expected: one commit containing the semaphore and its regression test.
