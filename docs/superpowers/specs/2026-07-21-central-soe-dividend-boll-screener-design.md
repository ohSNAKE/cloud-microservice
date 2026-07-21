# Central SOE Dividend BOLL Screener Design

## Goal

Add an A-share and Beijing Stock Exchange screener for large central state-owned enterprises that combine a high trailing dividend yield with a daily or weekly price near the lower Bollinger Band. The feature is informational only; it does not issue orders or provide trading instructions.

## Screening Rules

A stock is included only when every fundamental rule and at least one technical rule matches:

- Market: Shanghai and Shenzhen A shares, plus the Beijing Stock Exchange.
- Ownership: the actual controller is the State Council, SASAC, or a centrally administered state-owned enterprise. Local SOEs are excluded.
- Total market capitalization: at least CNY 50 billion.
- Dividend yield: at least 5%. Calculate it as the total cash dividend per share declared in the preceding calendar year divided by the current share price.
- Bollinger Bands: calculate `BOLL(20, 2)` from closing prices independently for daily and weekly K-lines. A period matches when `current price <= lower band * 1.02`; the daily or weekly period may match.

Stocks with missing ownership, market capitalization, dividend, price, or fewer than 20 completed bars for a required period are excluded. A zero dividend does not match the dividend rule.

## Architecture

Create a standalone Rust `screener` service and Tauri commands. It does not share watchlist membership, signal state, or strategy settings with the existing quant-alert service.

Extend the existing Eastmoney-backed quote transport with read-only public-data fetchers for:

- The A-share and Beijing Stock Exchange stock universe.
- Fundamental snapshots containing market capitalization and actual-controller information.
- Cash-dividend records for the preceding calendar year.
- Realtime stock quotes and daily/weekly K-lines.

The service normalizes the upstream data into an internal candidate snapshot. It filters ownership, market capitalization, and dividend yield before fetching K-lines for the remaining candidates. It calculates daily and weekly BOLL values locally and persists only qualifying results plus the refresh summary.

The frontend adds a dedicated `选股` route and API methods. It reads persisted results on load and requests a refresh through the new command when the user selects manual refresh.

## Storage And Refresh

Add SQLite tables for:

- A daily fundamental snapshot keyed by stock code, exchange, and China trading date.
- The latest screener results, including fundamental values, BOLL values, matching periods, per-field timestamps, and source metadata.
- A refresh-run summary containing start/end times, status, candidate and match counts, skipped-item counts, and a short failure summary.

The first refresh on a China trading date retrieves and stores the fundamental snapshot. Subsequent refreshes reuse it. During China trading hours (09:30-15:00), the application refreshes eligible candidates every 15 minutes while the screener page is active; manual refresh is always available. Outside trading hours, manual refresh may rebuild the daily fundamental snapshot and K-line values while retaining the last available quote where no newer quote exists.

Bound network concurrency across each fetch stage. Failed requests and partial records are not written as valid cached values. A failure for one stock is recorded as a skipped item and does not abort the run.

## UI And Interaction

Add a sidebar entry named `选股`.

The page contains:

- A compact status area with match count, latest successful refresh time, current refresh state, and a manual refresh button.
- A visible, read-only strategy summary: central SOE, market capitalization at least CNY 50 billion, dividend yield at least 5%, and within 2% of the daily or weekly BOLL lower band.
- A sortable results table. Default sort is ascending distance to the lower band. Columns include stock name, code, exchange, market capitalization, preceding-year cash dividend per share, dividend yield, current price, daily and weekly lower bands, distance to each lower band, matching period, and data timestamp.
- Per-row actions to open the existing K-line chart and add the stock to the quant-alert watchlist.
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
- Daily and weekly `BOLL(20, 2)` calculations, including the 2% lower-band boundary.
- Matching when either, both, or neither period meets the technical condition.

Rust service tests cover:

- Upstream response normalization.
- Daily fundamental-cache hits, trading-date invalidation, and failed-request non-caching.
- Bounded concurrency and partial-refresh completion.

Frontend tests cover:

- Result ordering and displayed technical labels.
- Empty, stale, failed, and refreshing states.
- Manual refresh and adding a matching result to the quant watchlist.

Run Rust tests, `npm test`, `npm run typecheck`, and `npm run build` before completion.
