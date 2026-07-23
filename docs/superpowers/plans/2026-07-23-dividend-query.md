# 全 A 股分红查询 Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在现有 Tauri 桌面应用中增加按东方财富行业或股票查询全 A 股分红、双口径股息率和可追溯历年方案的独立页面。

**Architecture:** Rust 先建立稳定的共享 DTO，再分别实现东方财富无损解析、行业/行情 source、分红领域聚合和可取消流式 runner。Tauri command 仅适配 IPC channel 与查询 registry；React 使用同构联合类型和纯状态机隔离过期事件，页面不直接访问第三方接口。

**Tech Stack:** Rust 2021、Tauri 2 IPC Channel、tokio、ureq、serde、chrono、React 19、TypeScript、Ant Design 5、Vitest。

**Design spec:** `docs/superpowers/specs/2026-07-23-dividend-query-design.md`

---

## Chunk 1: Provider And Domain

### Task 1: Establish Shared Dividend Query DTOs

**Files:**
- Create: `src-tauri/src/services/dividend_query_types.rs`
- Modify: `src-tauri/src/services/mod.rs`
- Test: `src-tauri/src/services/dividend_query_types.rs`

- [ ] **Step 1: Write failing serde-contract tests**

Define compile targets in tests for every DTO from the spec, then serialize one sample of each `DividendQueryEvent` and `DividendItemResult`. Assert snake_case tags, decimal yield values, `null` options, `SH | SZ | BJ`, RFC 3339 timestamps, and `YYYY-MM-DD` dates.

```rust
#[test]
fn serializes_item_event_contract() {
    let event = DividendQueryEvent::Item {
        query_id: "q-1".into(), completed: 1, total: 2,
        result: DividendItemResult::Failure {
            stock: stock("601600", DividendExchange::Sh),
            stage: DividendFailureStage::Dividend,
            code: "provider_timeout".into(),
            message: "timeout".into(),
        },
    };
    assert_eq!(serde_json::to_value(event).unwrap()["type"], "item");
}
```

- [ ] **Step 2: Run the contract test and verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml services::dividend_query_types::tests::serializes_item_event_contract -- --exact`

Expected: FAIL because the module and DTOs do not exist.

- [ ] **Step 3: Implement only the shared types**

Implement the spec's `DividendIndustry`, `DividendStockRef`, `DividendRecord`, `DividendPeriodMetrics`, `DividendTrailingMetrics`, `DividendStockSummary`, `DividendStockDetail`, `AnnouncementLink`, `CommandError`, `DividendQueryInput`, `DividendQueryEvent`, and result enums. Add internal-but-shared `DividendQuote { stock_key, price, observed_at, time_kind }`. This file contains no parsing, HTTP, calculations, Tauri types, mutexes, or async code.

Use tagged unions:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DividendQueryEvent {
    Started { query_id: String, total: usize, started_at: String },
    Item { query_id: String, completed: usize, total: usize, result: DividendItemResult },
    Finished { query_id: String, completed: usize, total: usize, succeeded: usize, failed: usize, finished_at: String },
    Cancelled { query_id: String, completed: usize, total: usize },
    Failed { query_id: String, code: String, message: String, retryable: bool },
}
```

Add constructors for stable `CommandError` codes: `validation`, `network`, `rate_limited`, `provider_server`, `provider_client`, `parse`, `channel_closed`, and `duplicate_query`.

- [ ] **Step 4: Run tests and formatting**

Run: `cargo test --manifest-path src-tauri/Cargo.toml services::dividend_query_types`

Expected: PASS.

Run: `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`

Expected: PASS after formatting if needed.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/services/mod.rs src-tauri/src/services/dividend_query_types.rs
git commit -m "feat(dividends): define query data contracts"
```

### Task 2: Extend The Existing Dividend Parser Without Regressions

**Files:**
- Modify: `src-tauri/src/services/screener_source_eastmoney_market.rs`
- Test: `src-tauri/src/services/screener_source_eastmoney_market.rs`

- [ ] **Step 1: Add a complete failing lossless-parser test**

Use two rows. The first supplies `SECUCODE`, optional `INFO_CODE`, all report/notice/plan/status/amount/record/ex-date fields. The second has null dates, amount, status, plan text, and info code. Assert every field exactly, including nulls. Preserve `INFO_CODE` as `announcement_id`, not `provider_record_id`: it identifies an announcement and revisions may receive new values. `SECURITY_INNER_CODE` is company-level and must not be treated as a plan ID either. Leave `provider_record_id` null unless Eastmoney later exposes a documented revision-stable plan identity. Preserve `SECUCODE` separately.

```rust
#[test]
fn parses_all_dividend_detail_fields_and_nulls() {
    let rows = parse_dividend_detail_json(DIVIDEND_DETAIL_FIXTURE).unwrap();
    assert_eq!(rows[0].secucode.as_deref(), Some("000807.SZ"));
    assert!(rows[0].provider_record_id.is_none());
    assert_eq!(rows[0].announcement_id.as_deref(), Some("AN202603271234"));
    assert_eq!(rows[0].report_date.as_deref(), Some("2025-12-31 00:00:00"));
    assert_eq!(rows[0].announcement_date.as_deref(), Some("2026-06-04 00:00:00"));
    assert_eq!(rows[0].plan_text.as_deref(), Some("10派3.79元(含税)"));
    assert_eq!(rows[0].cash_per_ten_shares, Some(3.79));
    assert_eq!(rows[0].assign_progress.as_deref(), Some("实施分配"));
    assert_eq!(rows[0].equity_record_date.as_deref(), Some("2026-06-09 00:00:00"));
    assert_eq!(rows[0].ex_dividend_date.as_deref(), Some("2026-06-10 00:00:00"));
    assert!(rows[1].report_date.is_none() && rows[1].ex_dividend_date.is_none());
}
```

- [ ] **Step 2: Run it and verify the missing symbol failure**

Run: `cargo test --manifest-path src-tauri/Cargo.toml services::screener_source_eastmoney_market::tests::parses_all_dividend_detail_fields_and_nulls -- --exact`

Expected: FAIL because `parse_dividend_detail_json` is absent.

- [ ] **Step 3: Add the provider row and parser**

Add public `EastmoneyDividendDetail` and `parse_dividend_detail_json`. The provider row owns only raw optional values. It may expose an allowlisted official URL if the payload explicitly provides one, but it retains `INFO_CODE` only as `announcement_id` for later link/search construction. Add a test proving two rows with different `INFO_CODE` values still have null stable plan IDs and therefore enter fallback grouping. Do not normalize status or calculate amounts here.

- [ ] **Step 4: Add a failing full regression test for the old parser**

Create expected `Vec<NormalizedDividend>` values for implemented, missing ex-date, malformed date, and proposal rows. Compare full structs, including code, exchange, ex-date sentinel behavior, source amount, adjustment, gross amount, and `confirmed_cash`.

Run: `cargo test --manifest-path src-tauri/Cargo.toml services::screener_source_eastmoney_market::tests::detail_refactor_preserves_normalized_dividends -- --exact`

Expected: FAIL until the old parser is refactored to consume lossless rows.

- [ ] **Step 5: Refactor and verify all existing market parser tests**

Map `parse_dividend_detail_json` rows into the existing `NormalizedDividend` exactly as before. Do not modify `NormalizedDividend`, screener thresholds, or date semantics.

Run: `cargo test --manifest-path src-tauri/Cargo.toml services::screener_source_eastmoney_market`

Expected: PASS, including all pre-existing quote, K-line, and dividend tests.

- [ ] **Step 6: Format and commit**

Run: `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`

Expected: PASS.

```bash
git add src-tauri/src/services/screener_source_eastmoney_market.rs
git commit -m "feat(dividends): preserve Eastmoney dividend fields"
```

### Task 3: Add A-Share Classification And Eastmoney Source

**Files:**
- Create: `src-tauri/src/services/dividend_query_source.rs`
- Create: `src-tauri/src/services/dividend_query_http.rs`
- Modify: `src-tauri/src/services/mod.rs`
- Test: `src-tauri/src/services/dividend_query_source.rs`
- Test: `src-tauri/src/services/dividend_query_http.rs`

- [ ] **Step 1: Write failing table tests for the A-share classifier**

Accept only six-digit ordinary A-share families using an exact rule: SH three-digit prefixes `{600, 601, 603, 605, 688}`; SZ three-digit prefixes `{000, 001, 002, 003, 300, 301}`; BJ first digit `4` or `8`, or two-digit prefix `92`. Include `900xxx`/`200xxx` B shares, `5xxxxx`/`1xxxxx` funds and bonds, `11xxxx`/`12xxxx` convertibles, `689xxx` depositary receipts, malformed codes, unsupported security type, and an explicit inactive status (when the suggestion payload supplies one) as rejected fixtures. Preserve other `9xxxxx` as rejected; add a regression note that this intentionally tightens the existing screener's broad `9` heuristic for this feature.

Run: `cargo test --manifest-path src-tauri/Cargo.toml services::dividend_query_source::tests::classifies_supported_a_shares -- --exact`

Expected: FAIL because the source module is absent.

- [ ] **Step 2: Implement classifier and pure endpoint parsers**

The industry endpoint requests `f12,f14`; constituent endpoint requests `f12,f13,f14`. Do not infer listing status from quote fields such as `f17` (opening price), because suspension is valid and is not delisting. The constituent endpoint is a current equity-scoped board list, so its rows require code-family validation but no price/status heuristic. The suggestion endpoint requests/retains `Code`, `Name`, `QuoteID`, `SecurityTypeName`, and a documented security status only if that endpoint supplies one; accept the actual Eastmoney A-share labels `{沪A, 深A, 京A, A股}` and require the label to agree with the normalized exchange. Reject explicit inactive/delisted status, but never reject for missing price. Add one accepted fixture for each provider label and one mismatch rejection.

Batch quote URLs must set `fltt=2` and request `f12,f2,f124`, so `f2` is parsed directly as yuan rather than cents. A valid positive finite `f2` creates a quote. Missing, non-positive, or malformed prices omit only that stock's quote rather than failing the batch. Positive `f124` is converted to RFC 3339 and `time_kind = provider`; missing/invalid timestamp uses the injected response `received_at` and `time_kind = received_at`. Missing/invalid stock code makes only that row unusable. Tests cover all four price/time combinations.

Implement and test:

```rust
pub fn classify_a_share(code: &str, market_id: Option<i64>, security_type: Option<&str>, active: bool)
    -> Option<DividendExchange>;
pub fn parse_industries_json(text: &str) -> Result<Vec<DividendIndustry>, CommandError>;
pub fn parse_constituents_json(industry: &DividendIndustry, text: &str)
    -> Result<Vec<DividendStockRef>, CommandError>;
pub fn parse_stock_search_json(text: &str) -> Result<Vec<DividendStockRef>, CommandError>;
pub fn parse_batch_quotes_json(text: &str, received_at: &str)
    -> Result<Vec<DividendQuote>, CommandError>;
```

- [ ] **Step 3: Add failing URL-contract tests**

Assert exact endpoint host, required fields, page size, industry ID placement, at most 100 `secids` per quote URL, and percent-encoded stock query. Assert empty/one-character searches fail validation and search results cap at 20.

Run: `cargo test --manifest-path src-tauri/Cargo.toml services::dividend_query_source::tests::builds_encoded_provider_urls -- --exact`

Expected: FAIL until URL builders exist.

- [ ] **Step 4: Implement source trait and URL builders**

`dividend_query_source.rs` owns the trait, classifier, URLs, and pure JSON parsers. It imports shared DTOs from Task 1 and raw dividend details from Task 2, so this task compiles independently.

```rust
pub trait DividendQuerySource: Send + Sync {
    fn list_industries(&self) -> Result<Vec<DividendIndustry>, CommandError>;
    fn list_constituents(&self, industry: &DividendIndustry) -> Result<Vec<DividendStockRef>, CommandError>;
    fn search_stocks(&self, query: &str) -> Result<Vec<DividendStockRef>, CommandError>;
    fn fetch_dividends(&self, stock: &DividendStockRef) -> Result<Vec<EastmoneyDividendDetail>, CommandError>;
    fn fetch_quotes(&self, stocks: &[DividendStockRef]) -> Result<Vec<DividendQuote>, CommandError>;
}
```

- [ ] **Step 5: Add failing HTTP mapping tests**

Use an injected getter response enum to test exact error mappings: transport/timeout -> `network` retryable; 429 -> `rate_limited` retryable; 500-599 -> `provider_server` retryable; other 400-499 -> `provider_client` non-retryable; JSON failure -> `parse` non-retryable. Test HTTPS-only rejection and required User-Agent/Referer headers.

- [ ] **Step 6: Implement the live HTTP adapter**

`dividend_query_http.rs` owns `EastmoneyDividendQuerySource<G>` and the production `ureq` getter. It composes Task 3 parsers and Task 2's dividend URL/parser. Set one 10-second request timeout; retry remains runner-owned. Keep the getter cloneable for tests.

- [ ] **Step 7: Run, format, and commit**

Run: `cargo test --manifest-path src-tauri/Cargo.toml services::dividend_query_source`

Expected: PASS.

Run: `cargo test --manifest-path src-tauri/Cargo.toml services::dividend_query_http`

Expected: PASS.

Run: `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`

Expected: PASS.

```bash
git add src-tauri/src/services/mod.rs src-tauri/src/services/dividend_query_source.rs src-tauri/src/services/dividend_query_http.rs
git commit -m "feat(dividends): add Eastmoney A-share source"
```

### Task 4: Implement Dividend Normalization And Metrics

**Files:**
- Create: `src-tauri/src/services/dividend_query_domain.rs`
- Modify: `src-tauri/src/services/mod.rs`
- Test: `src-tauri/src/services/dividend_query_domain.rs`

- [ ] **Step 1: Write failing explicit status/link tests**

Apply status mapping in strict precedence order: `取消`/`终止`/`否决`/`不分配` -> cancelled first; otherwise contains `实施` -> implemented; otherwise `股东大会` plus `通过` -> approved; otherwise `董事会预案`/`预案`/`预披露` -> proposal; otherwise unknown. Include `终止实施` and `取消实施` fixtures proving cancellation wins. Test latest effective status orders by announcement date, then report date, then ex-date descending and excludes cancelled/unknown. Test only parsed allowlisted official hosts become `original`; `INFO_CODE` produces an Eastmoney `search`; absent identifiers produce `none`.

Run: `cargo test --manifest-path src-tauri/Cargo.toml services::dividend_query_domain::tests::normalizes_status_and_latest_effective -- --exact`

Expected: FAIL because the domain module is absent.

- [ ] **Step 2: Implement record normalization only**

Implement status/link/date parsing and `normalize_records`. Preserve malformed records with a quality warning. Keep calculations for later steps.

- [ ] **Step 3: Write failing stable-ID and ambiguous-group tests**

Assert latest announcement wins for repeated provider ID; identical fallback rows collapse; differing no-ID rows in one report period remain in detail but all become ineligible with the exact warning from the spec; cancelled revisions do not aggregate.

- [ ] **Step 4: Implement deduplication and rerun normalization tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml services::dividend_query_domain::tests::deduplicates_dividend_records -- --exact`

Expected: PASS after minimal deduplication.

- [ ] **Step 5: Write failing annual metric tests**

Cover implemented/pending/disclosed state table, multiple distributions in one year, cross-year implementation remaining in report year, no eligible year returning null, query date `2026-07-23` excluding every `REPORT_DATE` in 2026, and a newer pre-2026 ambiguous-only year falling back with `quality_warnings`.

- [ ] **Step 6: Implement annual metrics with an explicit query date and verify**

The pure contract is `latest_annual_metrics(records: &[DividendRecord], query_date: NaiveDate, price: Option<f64>)`; it considers only eligible report years strictly less than `query_date.year()`.

Run: `cargo test --manifest-path src-tauri/Cargo.toml services::dividend_query_domain::tests::calculates_latest_annual_metrics -- --exact`

Expected: PASS.

- [ ] **Step 7: Write failing trailing/yield tests**

Use Shanghai query date fixtures for closed calendar-month boundary, leap day, missing ex-date exclusion, ineligible exclusion, and missing/zero/non-finite price returning null yield.

- [ ] **Step 8: Implement trailing metrics and summary composition**

Add pure `latest_annual_metrics`, `trailing_metrics`, and `build_summary`; no network, mutex, or Tauri imports.

- [ ] **Step 9: Run all Rust tests, format, and commit**

Run: `cargo test --manifest-path src-tauri/Cargo.toml services::dividend_query_domain`

Expected: PASS.

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: PASS with unchanged screener fixtures.

Run: `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`

Expected: PASS.

```bash
git add src-tauri/src/services/mod.rs src-tauri/src/services/dividend_query_domain.rs
git commit -m "feat(dividends): calculate dividend metrics"
```

## Chunk 2: Streaming Backend

### Task 5: Implement Retry And Per-Stock Coordination

**Files:**
- Create: `src-tauri/src/services/dividend_query_runner.rs`
- Modify: `src-tauri/src/services/mod.rs`
- Test: `src-tauri/src/services/dividend_query_runner.rs`

- [ ] **Step 1: Write failing input-resolution and started-event tests**

With a fake source/sink, assert stock mode resolves to one stock, industry mode fetches constituents, `started` is first and has the resolved total, an industry failure emits one `failed` terminal with no started/items, and one Shanghai query timestamp/date is reused across all summaries.

- [ ] **Step 2: Implement runner interfaces and resolution only**

Define Tauri-independent `DividendEventSink`, `DividendClock`, `RetrySleeper`, and `QueryControl`. `RetrySleeper::sleep` returns `Pin<Box<dyn Future<Output = ()> + Send>>`; tests use an immediate sleeper and fixed Shanghai clock. `QueryControl` already exposes `request_cancel`, `is_cancelled`, cancellation notification, and atomic `claim_finished | claim_failed`; its terminal value represents cancellation exactly as Task 6 specifies. Start with resolution plus terminal failure so all cancellation tests in this task compile.

- [ ] **Step 3: Write failing two-attempt retry tests**

Test dividend and quote independently: retryable first failure then success, retryable exhaustion after two calls, non-retryable one call, cancellation before attempt, cancellation during injected delay, and cancellation after operation before accepting result.

- [ ] **Step 4: Implement one shared retry helper**

Use exactly two attempts, one 500ms delay, and cancellation checks before call, before/after delay, and after call. The helper consumes `CommandError.retryable`; source parsing/4xx remain non-retryable.

- [ ] **Step 5: Write failing per-stock state tests**

Model each stock as `dividend: Pending | Ready(rows) | Failed(error)`, `quote: Pending | Ready(option)`, and `emitted: bool`. Assert dividend failure emits immediately once; dividend success waits for quote terminal; quote batch result distributes by stock key; missing quote becomes `Ready(None)`; repeated callbacks cannot duplicate item/progress.

- [ ] **Step 6: Implement the state coordinator only**

Add a private `StockWorkState::try_take_item()` pure method. Do not add task scheduling yet. This method is the sole owner of exactly-once item creation.

- [ ] **Step 7: Write failing bounded-scheduling tests**

Assert at most six dividend operations run concurrently, quotes chunk at 100, only six dividend jobs are initially scheduled, completion schedules the next job, sink failure/cancellation schedules no more, already-running blocking calls are ignored after return, and final progress equals total.

- [ ] **Step 8: Implement incremental scheduling**

Use `tokio::task::JoinSet` and `spawn_blocking`; never enqueue the full dividend universe behind a semaphore. Start quote chunks promptly and up to six dividend jobs. Join results update coordinator states and emit items. On sink failure, set a local stopped flag, request cancellation, stop scheduling, drain/join already-running tasks without sending any later item or terminal event, then return `channel_closed`.

- [ ] **Step 9: Run focused runner tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml services::dividend_query_runner::tests::coordinates_stock_results -- --exact`

Expected: PASS.

Run: `cargo test --manifest-path src-tauri/Cargo.toml services::dividend_query_runner::tests::limits_dividend_concurrency -- --exact`

Expected: PASS.

### Task 6: Implement Unique Terminal State And Registry

**Files:**
- Modify: `src-tauri/src/services/dividend_query_runner.rs`
- Create: `src-tauri/src/services/dividend_query_registry.rs`
- Modify: `src-tauri/src/services/mod.rs`
- Test: `src-tauri/src/services/dividend_query_runner.rs`
- Test: `src-tauri/src/services/dividend_query_registry.rs`

- [ ] **Step 1: Write deterministic terminal-transition tests**

Runner is the only terminal-event sender, but cancellation must atomically claim its terminal state. `request_cancel()` compare-exchanges `RUNNING -> CANCELLED` and notifies the runner only when it wins. `claim_finished()` and `claim_failed()` independently compare-exchange from `RUNNING`; therefore whichever operation claims first determines the terminal. Test cancel-before-finish -> cancelled; finish-before-cancel -> finished and later cancel no-op; failed-before-cancel -> failed; terminal sink send failure returns `channel_closed` without attempting another terminal.

- [ ] **Step 2: Complete and verify `QueryControl` transition methods**

Use the `AtomicU8` and `tokio::sync::Notify` control introduced in Task 5; the terminal value itself represents cancellation, so a separate cancellation bool cannot create a read-then-claim race. Complete any terminal-send helpers required by these tests. The main runner `select!` observes the cancellation notification while joining work. After any claim, runner reads the claimed terminal and is solely responsible for its one matching terminal send. If that send fails, state remains terminal and runner returns `Err(channel_closed)`.

- [ ] **Step 3: Add deterministic race verification**

Coordinate two tasks with barriers, not sleeps, and assert exactly one claim succeeds.

Run: `for i in {1..20}; do cargo test --manifest-path src-tauri/Cargo.toml services::dividend_query_runner::tests::terminal_claim_is_unique -- --exact || exit 1; done`

Expected: all 20 runs PASS.

- [ ] **Step 4: Write failing registry tests**

Test `insert_if_absent`, duplicate ID error, unknown/repeated cancel as successful no-op, identity-checked remove, and cleanup after finished/cancelled/failed/channel-error outcomes. Poisoned lock maps to stable internal error rather than panicking.

- [ ] **Step 5: Implement focused registry operations**

`DividendQueryRegistry` owns only `Mutex<HashMap<String, Arc<QueryControl>>>` with `insert_if_absent`, `request_cancel`, and `remove_if_same(query_id, &Arc<QueryControl>)`. It imports no Tauri API.

- [ ] **Step 6: Run runner/registry tests, format, and commit**

Run: `cargo test --manifest-path src-tauri/Cargo.toml services::dividend_query_runner`

Expected: PASS.

Run: `cargo test --manifest-path src-tauri/Cargo.toml services::dividend_query_registry`

Expected: PASS.

Run: `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`

Expected: PASS.

```bash
git add src-tauri/src/services/mod.rs src-tauri/src/services/dividend_query_runner.rs src-tauri/src/services/dividend_query_registry.rs
git commit -m "feat(dividends): stream cancellable query progress"
```

### Task 7: Expose Tauri Commands And Channel Contracts

**Files:**
- Create: `src-tauri/src/commands/dividends.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/src/commands/dividends.rs`

- [ ] **Step 1: Write failing helper tests for every return path**

Use injectable source/sink helpers to test validation before network, duplicate IDs, and registry cleanup after finish, cancel, delivered failed terminal, validation error, runner error, and channel failure. Assert cleanup uses control identity. Assert a delivered `finished | cancelled | failed` returns `Ok(())`; only validation/duplicate before streaming or interruption without delivered terminal returns `Err(CommandError)`.

- [ ] **Step 2: Add registry state and command helpers**

Add `dividend_queries: Arc<DividendQueryRegistry>` to `AppState`. A `start_with_source_and_sink` helper performs validate -> insert -> run -> identity cleanup on every exit. Production constructs one live `EastmoneyDividendQuerySource`; tests inject fake source/sink.

- [ ] **Step 3: Implement channel adapter and start/cancel commands**

`TauriDividendEventSink` wraps `tauri::ipc::Channel<DividendQueryEvent>`. `start_dividend_query` awaits the runner future so frontend command resolution can detect abnormal stream closure. `cancel_dividend_query` only requests cancellation; runner sends the unique terminal.

- [ ] **Step 4: Write and implement metadata/detail command tests**

Test list/search/detail helpers use `spawn_blocking`, apply the same two-attempt retry policy, and return source metadata. Detail fetch starts quote and dividend operations together, captures one Shanghai query time, waits for both terminal states, normalizes records, and returns `DividendStockDetail { summary, records, source_name: "东方财富", source_url }`. Missing quote is successful with null yield; dividend failure is an error.

- [ ] **Step 5: Register all five commands**

Export and register `list_dividend_industries`, `search_dividend_stocks`, `start_dividend_query`, `cancel_dividend_query`, and `get_dividend_stock_detail` in `commands/mod.rs` and `tauri::generate_handler!`.

- [ ] **Step 6: Verify and commit**

Run: `cargo test --manifest-path src-tauri/Cargo.toml commands::dividends`

Expected: PASS.

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: PASS.

Run: `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`

Expected: PASS.

```bash
git add src-tauri/src/commands/dividends.rs src-tauri/src/commands/mod.rs src-tauri/src/lib.rs
git commit -m "feat(dividends): expose streaming query commands"
```

## Chunk 3: Frontend Experience

### Task 8: Add TypeScript Contracts And Tauri API Tests

**Files:**
- Modify: `src/types/index.ts`
- Modify: `src/api/index.ts`
- Create: `src/api/index.test.ts`

- [ ] **Step 1: Add compile-time DTO fixtures**

Mirror Rust serde types exactly. Add fixtures using `satisfies DividendQueryEvent`; do not add a runtime schema library. Add an exhaustive `switch` helper whose default assigns to `never`, covering every event and item variant.

- [ ] **Step 2: Write failing mocked Channel tests**

Mock `Channel` and `invoke` from `@tauri-apps/api/core` so tests do not require `window.__TAURI_INTERNALS__`. Assert `startDividendQuery` invokes `start_dividend_query` with `{ input, onEvent: channel }`, forwards `channel.onmessage`, and cancel uses `{ queryId }`. Test all five wrapper command names and argument casing.

- [ ] **Step 3: Run and verify failure**

Run: `npm test -- src/api/index.test.ts`

Expected: FAIL because dividend API wrappers/types are absent.

- [ ] **Step 4: Implement DTOs and wrappers**

Import `Channel` from `@tauri-apps/api/core`. Keep the channel alive for the command promise duration and return that promise to the page.

- [ ] **Step 5: Verify and commit**

Run: `npm test -- src/api/index.test.ts`

Expected: PASS.

Run: `npm run typecheck`

Expected: PASS.

```bash
git add src/types/index.ts src/api/index.ts src/api/index.test.ts
git commit -m "feat(dividends): add frontend query contracts"
```

### Task 9: Build A Pure Query Lifecycle And View Model

**Files:**
- Create: `src/pages/dividendQueryView.ts`
- Create: `src/pages/dividendQueryView.test.ts`

- [ ] **Step 1: Write failing lifecycle tests**

Test every event variant for stale query ID rejection; duplicate item replacement without progress inflation; no event mutation after any terminal; cancel retains rows and is non-error; command resolve/reject without matching terminal marks interrupted and retains rows; resolve/reject after terminal is no-op. `commandSettled(queryId, outcome)` must ignore a replaced query's late promise settlement. Test synchronous `beginReplacement` invalidates old ID before cancellation promise starts, and cancel rejection cannot reactivate it. Test unmount/detail-close action requests cancel when active.

- [ ] **Step 2: Run lifecycle tests and verify the RED state**

Run: `npm test -- src/pages/dividendQueryView.test.ts -t "query lifecycle"`

Expected: FAIL because the lifecycle reducer is absent.

- [ ] **Step 3: Implement lifecycle reducer only**

Use `DividendQueryState { activeQueryId, phase, rowsByKey, completed, total, error, warning }`. Export pure actions for begin, event, command settled, unmount, and detail close. Events are accepted only in `running` phase with matching ID.

- [ ] **Step 4: Write failing query-mode and sort/filter tests**

Cover invalid-condition query disabling; changing industry/stock query mode clears the incompatible condition; annual disclosed yield; trailing implemented yield; per-share dividend; stock name; null-last behavior in both directions; composed yield/status filters; and annual/trailing metric-mode switch resetting any user sort to the corresponding descending default and selecting the active emphasis column.

- [ ] **Step 5: Run view tests and verify the second RED state**

Run: `npm test -- src/pages/dividendQueryView.test.ts -t "query controls and sorting"`

Expected: FAIL because selectors and mode transitions are absent.

- [ ] **Step 6: Implement selectors and formatters**

Export `visibleDividendRows`, `setMetricMode`, `formatDividendYield`, date/time formatting, and announcement label selection. Keep React, Tauri, and Ant Design imports out.

- [ ] **Step 7: Write failing presentation-state tests**

Cover industry-list error/retry, stock-search error retaining input, empty result, progress, row failure, page failed/retry, all-dividend-failed warning, all-quotes-missing warning, cancelled, detail loading/error, exact Eastmoney disclosure, separate quote/dividend timestamps, and `original | search | none` labels.

- [ ] **Step 8: Run presentation tests and verify the third RED state**

Run: `npm test -- src/pages/dividendQueryView.test.ts -t "presentation states"`

Expected: FAIL because presentation derivation is absent.

- [ ] **Step 9: Implement presentation-state derivation and verify**

Run: `npm test -- src/pages/dividendQueryView.test.ts`

Expected: PASS.

Run: `npm run typecheck`

Expected: PASS.

- [ ] **Step 10: Commit**

```bash
git add src/pages/dividendQueryView.ts src/pages/dividendQueryView.test.ts
git commit -m "feat(dividends): model dividend query lifecycle"
```

### Task 10: Build The Dividend Query Page

**Files:**
- Create: `src/pages/Dividends.tsx`
- Modify: `src/styles/global.css`

The repository has no DOM renderer. Do not add `Dividends.test.tsx` or a new test framework; lifecycle and presentation behavior stays in Task 9 pure tests. Component wiring is verified by typecheck, build, and manual Tauri testing.

- [ ] **Step 1: Implement controls and metadata states**

Use `PageHeader`, a query-mode `Segmented` (`按行业 | 按股票`), a metric-mode `Segmented` (`最近完整年度 | 近 12 个月`), searchable `Select`, `AutoComplete`, numeric yield filters, status select, query button, retry alerts, and empty state. Mode changes use Task 9 helpers to clear incompatible conditions; invalid conditions disable query. Initial render may load industries but never dividends. Stock suggestions require two characters and 300ms debounce; each candidate shows name, code, and exchange so same-name securities remain distinguishable. Search errors retain text. Only clicking the enabled query button may start a dividend-fetch command; selecting an industry/candidate never starts one.

- [ ] **Step 2: Wire cancellation-safe query lifecycle**

Before awaiting old cancellation, synchronously dispatch `beginReplacement` to invalidate its ID. Cancellation rejection becomes a warning only. Generate `crypto.randomUUID()`, attach channel listener, and pass command settle results into the reducer. On unmount and active-query detail close, invalidate first then request cancellation. Late/duplicate/post-terminal events are rejected by Task 9 reducer.

- [ ] **Step 3: Implement progress, error, and warning states**

Render X/Y progress, row failures, page terminal failure with retry, abnormal interruption retaining partial rows, all-dividend-failed warning, all-quotes-missing warning, user cancellation without error, and empty completed result. Do not collapse these into one generic alert.

- [ ] **Step 4: Implement compact results table**

Columns: stock, industry, current price/time, annual disclosed dividend/yield, annual implemented/pending split, trailing dividend/yield, latest status/ex-date, fetch time, detail. Use stable widths and horizontal scrolling. Apply a restrained active-column class based on metric mode; switching mode invokes Task 9's default-sort reset. Null is `--`; warnings use tooltip/tag without changing row height.

- [ ] **Step 5: Implement detail drawer**

Show loading/error/retry, summary, history, quality warnings, exact “分红与行情数据来源：东方财富公开数据接口”, `source_name`/`source_url`, separate quote/dividend timestamps, and correct `公告原文`/`公告检索`/no-link states.

- [ ] **Step 6: Add responsive scoped styles**

Use `.dividend-query-*`, existing tokens, max 8px radii, stable table/control dimensions, and wrapping under 900px. No decorative gradients/cards or viewport-scaled fonts.

- [ ] **Step 7: Verify and commit**

Run: `npm test -- src/pages/dividendQueryView.test.ts src/api/index.test.ts`

Expected: PASS.

Run: `npm run typecheck && npm run build`

Expected: PASS.

```bash
git add src/pages/Dividends.tsx src/styles/global.css
git commit -m "feat(dividends): build dividend query workspace"
```

### Task 11: Add Navigation, Live Smoke Test, And Final Verification

**Files:**
- Modify: `src/App.tsx`
- Create: `src-tauri/tests/dividend_live_api_test.rs`
- Modify: `README.md`

Route wiring has no DOM test infrastructure and is explicitly verified by typecheck/build/manual navigation rather than introducing a framework for one route.

- [ ] **Step 1: Add route and menu wiring**

Use an existing Ant Design money/distribution icon, place `/dividends` beside `/screener`, add route metadata, import `DividendsPage`, and register the route.

- [ ] **Step 2: Add ignored live API smoke tests**

Test industry list, one industry's constituents, one stock's dividends, and one quote. Mark `#[ignore]`; assert shape/identifiers only, never exact price/payout.

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test dividend_live_api_test -- --ignored --nocapture`

Expected: PASS when Eastmoney is reachable. If unavailable, record the network error; do not weaken deterministic tests.

- [ ] **Step 3: Update README source caveat**

Document full A-share dividend query and that Eastmoney is a third-party aggregation source with manual announcement verification links.

- [ ] **Step 4: Run all automated verification**

Run: `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`

Expected: PASS.

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: PASS.

Run: `npm test`

Expected: PASS.

Run: `npm run typecheck && npm run build`

Expected: PASS.

- [ ] **Step 5: Run and manually verify the Tauri app**

Run: `npm run tauri dev`

Verify no initial dividend request; industry/stock paths; X/Y completion; stale-query isolation; annual/trailing default sort; composed filters; detail/source/link labels; partial failures; desktop and narrow-window overlap; and menu navigation.

- [ ] **Step 6: Inspect and commit integration only**

Run: `git status --short` and `git diff --check`.

```bash
git add src/App.tsx src-tauri/tests/dividend_live_api_test.rs README.md
git commit -m "feat(dividends): expose dividend query page"
```

Do not stage, revert, or reformat unrelated pre-existing workspace changes.
