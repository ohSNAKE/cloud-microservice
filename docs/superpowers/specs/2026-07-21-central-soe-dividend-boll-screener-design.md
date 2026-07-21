# Central SOE Dividend BOLL Screener Design

## Goal

Add an A-share and Beijing Stock Exchange screener for large central state-owned enterprises that combine a high trailing dividend yield with a daily or weekly price near the lower Bollinger Band. The feature is informational only; it does not issue orders or provide trading instructions.

## Screening Rules

A stock is included only when every fundamental rule and at least one technical rule matches:

- Market: Shanghai and Shenzhen A shares, plus the Beijing Stock Exchange.
- Ownership: the actual controller is the State Council, SASAC, or a centrally administered state-owned enterprise. Local SOEs are excluded. The authority is the public SASAC central-enterprise directory, cached by China valuation date. A record is `central_soe` only when its normalized actual-controller name equals either the direct-controller allow-list (`国务院`, `国务院国有资产监督管理委员会`) or a normalized current/historical alias in that directory. Normalization applies Unicode NFKC, trims whitespace, and removes only `（集团）有限公司`, `集团有限公司`, `有限公司`, and `集团` suffixes. No fuzzy or substring match is permitted.
- Total market capitalization: at least CNY 50 billion.
- Dividend yield: at least 5%. The China valuation date is the Asia/Shanghai ISO calendar date when the run begins, including weekends and holidays; its preceding calendar year is the dividend year. Calculate yield as the sum of gross, pre-withholding-tax cash dividends per current ordinary share whose ex-dividend date falls in that year, divided by the current share price. Include special cash dividends when the source identifies them as a per-share cash distribution. Exclude non-cash distributions, unconfirmed records, and records without an ex-dividend date. The dividend transport reads the provider's gross cash dividend per 10 shares and ex-dividend date, normalizes it to `gross_per_ex_date_share = cash_per_10 / 10`, and reads the provider's total-share-capital history on the ex-dividend date and valuation date. When both capital values are available, normalize to `gross_per_current_share = gross_per_ex_date_share * ex_date_total_capital / valuation_date_total_capital` with `adjusted` status. When the provider explicitly states that its per-share amount already uses the valuation-date share basis, preserve it with `verified_unadjusted` status. Otherwise use `unverifiable`; any such confirmed cash-dividend record makes the stock's annual dividend total ineligible rather than silently omitting the record.
- Bollinger Bands: calculate `BOLL(20, 2)` from the 20 most recent completed, unadjusted closing bars independently for daily and weekly K-lines. The bars must use the same unadjusted price basis as the realtime quote. Let `m` be the arithmetic mean of the 20 closes and `s = sqrt(sum((close - m)^2) / 20)` be their population standard deviation; the lower band is `m - 2 * s`. Use the provider's completed end-of-week bars. Do not include an in-progress daily or weekly bar. A period matches when `current price <= lower band * 1.02`; the daily or weekly period may match.

Stocks with missing ownership, market capitalization, dividend, or price are excluded. A zero dividend does not match the dividend rule. Current price and lower-band values must be finite and strictly positive. A stock needs 20 completed bars for at least one period: unavailable daily data does not exclude a valid weekly match, and unavailable weekly data does not exclude a valid daily match. The distance for each available period is `(current price - lower band) / lower band`; unavailable periods display as `--`. For the default ordering, use the smaller available distance, then canonical code, then exchange.

## Architecture

Create a standalone Rust `screener` service and Tauri commands. It does not share watchlist membership, signal state, or strategy settings with the existing quant-alert service.

Extend the existing Eastmoney-backed quote transport with read-only public-data fetchers for:

- The A-share and Beijing Stock Exchange stock universe.
- Fundamental snapshots containing market capitalization and actual-controller information.
- Cash-dividend records for the preceding calendar year.
- Realtime stock quotes and daily/weekly K-lines.

The transport normalizes every source response into a `ScreenerUniverseRecord` with a canonical code, exchange (`sh`, `sz`, or `bj`), stock name, Eastmoney security identifier, ordinary-equity flag, CNY total market capitalization, controller classification (`central_soe`, `local_soe`, `non_soe`, or `unknown`), and actual-controller text. The code/exchange pair is the immutable identity; stock name is display metadata and can change without creating a new row. It admits only ordinary A-share and BSE equity records with CNY values; funds, bonds, indices, B shares, and unclassified securities are excluded. Only `central_soe` records can pass ownership filtering. The SASAC controller registry normalizes into legal name, historical aliases, and source date. Dividend records normalize to code, exchange, gross cash dividend per current ordinary share, ex-dividend date, distribution kind, confirmation state, source cash-per-10 value, ex-date/valuation-date total share capital, and adjustment status (`adjusted`, `verified_unadjusted`, or `unverifiable`).

Each exchange universe fetch is paginated. The transport obtains the provider's declared total page count, fetches every page, rejects a response with an inconsistent total or missing page, and deduplicates records by `(code, exchange)`. Duplicate records must have equal security identifiers; otherwise the exchange list fails. One page failure invalidates the full exchange list. A usable universe requires all three fully paginated exchange lists and at least one normalized ordinary-equity entry in each.

The quote contract is `(code, exchange, price, observed_at)` where price is finite, positive, unadjusted CNY and `observed_at` is a UTC RFC 3339 timestamp. A K-line contract is `(code, exchange, period, completed_at, close)` where period is `day` or `week`, close is finite, positive, unadjusted CNY, and `completed_at` is a UTC RFC 3339 timestamp. Before selecting bars, reject records with duplicate `completed_at`, invalid timestamps, or nonascending timestamps; then select the 20 latest completed bars. Every normalized value joins on `(code, exchange)`.

The service filters ownership, market capitalization, and dividend yield before fetching K-lines for the remaining candidates. It calculates daily and weekly BOLL values locally, persists every valid fundamental snapshot, and separately persists only qualifying result rows plus the refresh summary. Source-specific endpoint paths and field aliases are encapsulated within the transport; the normalized contract is the sole input consumed by the screener service.

The Tauri API exposes `get_screener_dashboard()`, `refresh_screener()`, and `get_screener_refresh_schedule()`. The dashboard response has `displayed_run` (nullable `{ id, status, started_at, completed_at, candidate_count, match_count, skipped_count, failure_summary }`), `latest_failed_attempt` (nullable `{ started_at, completed_at, failure_summary }`), `is_stale`, and `results`. Run status is `success` or `partial`; `matched_periods` is a nonempty array of `day` and/or `week`. `failure_summary` is nullable `{ code, message, skipped_count }`, where code is `universe_unavailable`, `provider_unavailable`, `partial_data`, `import_invalidated`, or `database_error`. Each result has `{ code, exchange, name, market_cap_cny, cash_dividend_per_share, dividend_yield, current_price, price_observed_at, daily_lower_band, daily_distance, daily_kline_completed_at, weekly_lower_band, weekly_distance, weekly_kline_completed_at, matched_periods, fundamental_observed_at }`, with unavailable period values as `null` and all timestamps as UTC RFC 3339. The schedule response has `{ should_refresh_now, next_refresh_at }`, resolved by the backend market-calendar service. The refresh command returns the completed or stale-marked dashboard. A process-local single-flight guard prevents overlapping automatic and manual refreshes; a manual request while one is running receives the in-flight result, while the UI disables the action and shows the running state.

The frontend adds a dedicated `选股` route and API methods. It reads persisted results on load and requests a refresh through the new command when the user selects manual refresh.

## Storage And Refresh

Add SQLite tables for:

- A daily fundamental snapshot keyed by stock code, exchange, China valuation date, and dividend calendar year. It contains only successfully normalized fundamental and dividend values.
- Screener results keyed by run identifier, stock code, and exchange, including fundamental values, BOLL values, matching periods, per-field timestamps, and source metadata.
- A refresh-run summary keyed by run identifier, with start/end UTC timestamps, status, candidate and match counts, skipped-item counts, and a short failure summary.

Fundamental snapshots have the unique key `(code, exchange, valuation_date, dividend_year)`. A successful or partial run atomically writes its summary and fully evaluated matching rows under one new run identifier. The dashboard returns the most recent successful or partial run and its identically keyed result rows; when the latest attempt failed, it also returns that failure summary and marks the displayed result stale. A run with per-stock skips is `partial` and contains only matches that were fully evaluated; its summary makes omissions visible. A run that cannot obtain a usable universe or cannot complete its database transaction is `failed` and retains the preceding successful or partial result rows unchanged.

The first refresh on a China valuation date retrieves and stores each successful fundamental snapshot. Subsequent refreshes reuse only successful per-stock snapshots; missing or failed entries are retried on every same-day run. Snapshot records are committed independently after their per-stock normalization succeeds, so they remain reusable even if a later result-run transaction fails. Result rows and their run summary remain atomic.

The frontend owns the page-lifetime timer and obtains the market-calendar decision from `get_screener_refresh_schedule()`. On page mount, it loads the dashboard and requests one refresh only when `should_refresh_now` is true. It schedules exactly one timer for `next_refresh_at`, then obtains a new schedule after each attempt. The backend resolves China exchange holidays and session boundaries, aligns next triggers to the first 15-minute boundary at or after 09:30, and returns no automatic trigger during the midday break, weekends, or holidays. Multiple windows may schedule simultaneously; the backend single-flight guard joins them into one run. Unmounting clears the frontend timer but does not cancel an already-started backend refresh. Manual refresh is always available. Outside trading hours, manual refresh may rebuild the daily fundamental snapshot and K-line values while retaining the last available quote where no newer quote exists.

Bound network concurrency across each fetch stage. Failed requests and partial records are not written as valid cached values. A failure for one stock is recorded as a skipped item and does not abort the run. A universe is usable only when each of the Shanghai, Shenzhen, and Beijing ordinary-equity lists is fetched and normalized with at least one entry; otherwise the run is failed. Operational failures return a failed, stale-marked dashboard rather than a Tauri command error; command errors are reserved for invalid input or local database failures.

Keep seven China valuation dates of fundamental snapshots and the 30 most recent result-run summaries/results; prune older derived rows after each completed run. Migration creates the new tables. Screener snapshots and results are derived cache data, so export omits them and import clears them before the next refresh. Import increments a persisted screener-data generation and clears all screener tables inside its import transaction. A refresh captures that generation before fetching and must verify it before every snapshot or result write; a changed generation discards the in-flight data and returns the post-import empty dashboard.

## UI And Interaction

Add a sidebar entry named `选股`.

The page contains:

- A compact status area with match count, latest successful refresh time, current refresh state, and a manual refresh button.
- A visible, read-only strategy summary: central SOE, market capitalization at least CNY 50 billion, dividend yield at least 5%, and within 2% of the daily or weekly BOLL lower band.
- A sortable results table. Default sort is ascending distance to the lower band. Columns include stock name, code, exchange, market capitalization, preceding-year cash dividend per share, dividend yield, current price, daily and weekly lower bands, distance to each lower band, matching period, and data timestamp.
- Per-row actions to open the existing K-line chart and add the stock to the quant-alert watchlist. This is a one-way integration: `sh`, `sz`, and `bj` results map to the existing `market: "cn"` watchlist representation, with the canonical code and current name. The existing Chinese-market quote transport derives the exchange from the stored code, including BSE codes. Adding an existing `(code, "cn")` watchlist item is a successful no-op; the UI reports whether it was added or already present. This action does not alter the screener result.
- Distinct first-use, no-match, stale-result, and refresh-failure states. A cached successful result remains visible after refresh failure with its timestamp and error summary.

## Error Handling And Data Quality

- Validate upstream response shape and numeric fields before calculation.
- Treat controller strings as an upstream normalized classification rather than loose frontend text matching.
- Exclude stocks that cannot establish a valid preceding-year cash-dividend total or a current usable price.
- Keep per-stock errors out of the result table while exposing their count in the refresh summary.
- Do not create trading signals, notifications, or broker integrations from screener matches.

## Testing

Rust unit tests cover:

- Central-SOE classification and local-SOE exclusion.
- CNY 50 billion market-capitalization and 5% dividend-yield boundaries.
- SASAC controller-registry alias matching and rejection of fuzzy matches.
- Preceding-year cash-dividend aggregation, cash-per-10-share normalization, share-capital adjustment, and invalid-data exclusion.
- Daily and weekly `BOLL(20, 2)` calculations, including unadjusted values, population deviation, zero/negative input rejection, and the 2% lower-band boundary.
- Matching when either, both, or neither period meets the technical condition.

Rust service tests cover:

- Upstream response normalization.
- Daily fundamental-cache hits, trading-date invalidation, and failed-request non-caching.
- Paginated-universe completeness, page failure, and duplicate identity handling.
- Bounded concurrency, partial-refresh completion, snapshot persistence after later failure, and atomic result replacement after partial and failed runs.
- Holiday-date resolution, schedule alignment, page lifecycle, and single-flight behavior across multiple windows.
- Database migration, retention pruning, import cache clearing, and import-versus-refresh generation races.

Frontend tests cover:

- Result ordering and displayed technical labels.
- Empty, stale, failed, and refreshing states.
- Manual refresh, schedule rearming, and adding a matching result to the quant watchlist with the exchange-to-market mapping.

Run Rust tests, `npm test`, `npm run typecheck`, and `npm run build` before completion.
