# K-Line Intraday Period Design

## Goal

Add a `分时` period option to the existing K-line modal for stocks and quant targets.

## Approved Approach

Expose the existing backend `5m` K-line support in the frontend period selector. The K-line modal will show period buttons in this order: `分时 / 日K / 周K / 月K`.

## Scope

- Extend frontend `KlinePeriod` type with `5m`.
- Add `分时` option in `KlineChart` for stock holdings and quant target chart entries.
- Keep fund charts unchanged as daily NAV line charts.
- Reuse existing backend `get_kline_data` and `fetch_stock_kline("5m")` support.

## Verification

- `npm run typecheck`
- `npm run build`
- Optional live backend check: existing `live_fetch_stock_five_minute_kline_002796` passes.
