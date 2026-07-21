# Central SOE Dividend BOLL Screener Design

## Goal

Add an A-share and Beijing Stock Exchange screener for large central state-owned enterprises that combine a high trailing dividend yield with a daily or weekly price near the lower Bollinger Band. The feature is informational only; it does not issue orders or provide trading instructions.

## Screening Rules

A stock is included only when every fundamental rule and at least one technical rule matches:

- Market: Shanghai and Shenzhen A shares, plus the Beijing Stock Exchange.
- Ownership: the actual controller is the State Council, SASAC, or a centrally administered state-owned enterprise. Local SOEs are excluded.
- Total market capitalization: at least CNY 50 billion.
- Dividend yield: at least 5%. The preceding calendar year is `China trading date at run start - 1`, including a run during the New Year holiday. Calculate it as the sum of all ordinary and interim cash dividends per share whose ex-dividend date falls in that year, divided by the current share price. Include special cash dividends when the source identifies them as a per-share cash distribution. Exclude non-cash distributions, unconfirmed records, and records without an ex-dividend date. A confirmed cash-dividend record must carry a source-adjusted per-share amount or a verified no-adjustment status; any other adjustment status makes the stock's annual dividend total ineligible rather than silently omitting the record.
- Bollinger Bands: calculate `BOLL(20, 2)` from the 20 most recent completed, unadjusted closing bars independently for daily and weekly K-lines. The bars must use the same unadjusted price basis as the realtime quote. Let `m` be the arithmetic mean of the 20 closes and `s = sqrt(sum((close - m)^2) / 20)` be their population standard deviation; the lower band is `m - 2 * s`. Use the provider's completed end-of-week bars. Do not include an in-progress daily or weekly bar. A period matches when `current price <= lower band * 1.02`; the daily or weekly period may match.

Stocks with missing ownership, market capitalization, dividend, or price are excluded. A zero dividend does not match the dividend rule. Current price and lower-band values must be finite and strictly positive. A stock needs 20 completed bars for at least one period: unavailable daily data does not exclude a valid weekly match, and unavailable weekly data does not exclude a valid daily match. The distance for each available period is `(current price - lower band) / lower band`; unavailable periods display as `--`. For the default ordering, use the smaller available distance, then canonical code, then exchange.

## Architecture

Create a standalone Rust `screener` service and Tauri commands. It does not share watchlist membership, signal state, or strategy settings with the existing quant-alert service.

Extend the existing Eastmoney-backed quote transport with read-only public-data fetchers for:

- The A-share and Beijing Stock Exchange stock universe.
- Fundamental snapshots containing market capitalization and actual-controller information.
- Cash-dividend records for the preceding calendar year.
- Realtime stock quotes and daily/weekly K-lines.

The transport normalizes every source response into a `ScreenerUniverseRecord` with a canonical code, exchange (`sh`, `sz`, or `bj`), Eastmoney security identifier, ordinary-equity flag, CNY total market capitalization, controller classification (`central_soe`, `local_soe`, `non_soe`, or `unknown`), and actual-controller text. It admits only ordinary A-share and BSE equity records with CNY values; funds, bonds, indices, B shares, and unclassified securities are excluded. Only `central_soe` records can pass ownership filtering. Dividend records normalize to code, exchange, cash dividend per share, ex-dividend date, distribution kind, confirmation state, and adjustment status (`adjusted`, `verified_unadjusted`, or `unverifiable`).

The quote contract is `(code, exchange, price, observed_at)` where price is finite, positive, unadjusted CNY and `observed_at` is a UTC RFC 3339 timestamp. A K-line contract is `(code, exchange, period, completed_at, close)` where period is `day` or `week`, close is finite, positive, unadjusted CNY, and `completed_at` is a UTC RFC 3339 timestamp. Every normalized value joins on `(code, exchange)`.

The service filters ownership, market capitalization, and dividend yield before fetching K-lines for the remaining candidates. It calculates daily and weekly BOLL values locally, persists every valid fundamental snapshot, and separately persists only qualifying result rows plus the refresh summary. Source-specific endpoint paths and field aliases are encapsulated within the transport; the normalized contract is the sole input consumed by the screener service.

The Tauri API exposes `get_screener_dashboard()` and `refresh_screener()`. The dashboard returns the latest completed run summary and its result rows. Each row contains identity, fundamental values, current price, daily/weekly lower bands and distances, matching periods, and data timestamps. The refresh command returns the completed dashboard. A process-local single-flight guard prevents overlapping automatic and manual refreshes; a manual request while one is running receives the in-flight result, while the UI disables the action and shows the running state.

The frontend adds a dedicated `选股` route and API methods. It reads persisted results on load and requests a refresh through the new command when the user selects manual refresh.

## Storage And Refresh

Add SQLite tables for:

- A daily fundamental snapshot keyed by stock code, exchange, China trading date, and dividend calendar year. It contains only successfully normalized fundamental and dividend values.
- Screener results keyed by run identifier, stock code, and exchange, including fundamental values, BOLL values, matching periods, per-field timestamps, and source metadata.
- A refresh-run summary keyed by run identifier, with start/end UTC timestamps, status, candidate and match counts, skipped-item counts, and a short failure summary.

Fundamental snapshots have the unique key `(code, exchange, trading_date, dividend_year)`. A successful or partial run atomically writes its summary and fully evaluated matching rows under one new run identifier. The dashboard returns the most recent successful or partial run and its identically keyed result rows; when the latest attempt failed, it also returns that failure summary and marks the displayed result stale. A run with per-stock skips is `partial` and contains only matches that were fully evaluated; its summary makes omissions visible. A run that cannot obtain a usable universe or cannot complete its database transaction is `failed` and retains the preceding successful or partial result rows unchanged.

The first refresh on a China trading date retrieves and stores each successful fundamental snapshot. Subsequent refreshes reuse only successful per-stock snapshots; missing or failed entries are retried on every same-day run. The frontend owns the page-lifetime timer and invokes the backend command only while the screener page is mounted. Automatic refresh runs in the China equity sessions 09:30-11:30 and 13:00-15:00, every 15 minutes. It never starts during the midday break, weekends, or exchange holidays determined by the market-calendar service. Unmounting clears the timer but does not cancel an already-started backend refresh. Manual refresh is always available. Outside trading hours, manual refresh may rebuild the daily fundamental snapshot and K-line values while retaining the last available quote where no newer quote exists.

Bound network concurrency across each fetch stage. Failed requests and partial records are not written as valid cached values. A failure for one stock is recorded as a skipped item and does not abort the run. A universe is usable only when each of the Shanghai, Shenzhen, and Beijing ordinary-equity lists is fetched and normalized with at least one entry; otherwise the run is failed. Operational failures return a failed, stale-marked dashboard rather than a Tauri command error; command errors are reserved for invalid input or local database failures. Migration creates the new tables. Screener snapshots and results are derived cache data, so export omits them and import clears them before the next refresh.

## UI And Interaction

Add a sidebar entry named `选股`.

The page contains:

- A compact status area with match count, latest successful refresh time, current refresh state, and a manual refresh button.
- A visible, read-only strategy summary: central SOE, market capitalization at least CNY 50 billion, dividend yield at least 5%, and within 2% of the daily or weekly BOLL lower band.
- A sortable results table. Default sort is ascending distance to the lower band. Columns include stock name, code, exchange, market capitalization, preceding-year cash dividend per share, dividend yield, current price, daily and weekly lower bands, distance to each lower band, matching period, and data timestamp.
- Per-row actions to open the existing K-line chart and add the stock to the quant-alert watchlist. This is a one-way integration: it creates an A-share watchlist item using the row's canonical code and name, does not alter the screener result, and treats an existing same-code/same-market item as a successful no-op.
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
- Preceding-year cash-dividend aggregation and invalid-data exclusion.
- Daily and weekly `BOLL(20, 2)` calculations, including unadjusted values, population deviation, zero/negative input rejection, and the 2% lower-band boundary.
- Matching when either, both, or neither period meets the technical condition.

Rust service tests cover:

- Upstream response normalization.
- Daily fundamental-cache hits, trading-date invalidation, and failed-request non-caching.
- Bounded concurrency and partial-refresh completion.
- Atomic result replacement after partial and failed runs, scheduler lifecycle, single-flight refresh behavior, database migration, and import cache clearing.

Frontend tests cover:

- Result ordering and displayed technical labels.
- Empty, stale, failed, and refreshing states.
- Manual refresh and adding a matching result to the quant watchlist.

Run Rust tests, `npm test`, `npm run typecheck`, and `npm run build` before completion.
