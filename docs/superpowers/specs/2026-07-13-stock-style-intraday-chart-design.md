# Stock-Style Intraday Chart Design

## Goal

Replace the current `分时` 5-minute candlestick view with a stock-software-style, single-day intraday chart. It must show the current price path, the previous-close reference, 5-minute volume, and optional intraday-T attention windows.

## Decisions

The first version combines the selected classic and quantitative-overlay directions:

- Use a continuous price line instead of candlesticks for the `分时` period.
- Display only the latest trading-date 5-minute bars. Never combine multiple dates into one intraday chart. During market hours, the current date is the latest trading date and its in-progress line is shown; otherwise the newest returned date is shown.
- Render a previous-close horizontal reference line.
- Show a price axis and an aligned percentage-to-previous-close axis.
- Render a separate lower volume panel. Each 5-minute bar uses the existing rise/fall colors.
- Label returned intraday bars at `09:30`, `11:30`, `13:00`, and `15:00`; a session label whose bar is absent from a partial or delayed day is omitted. The x-axis contains only returned bar times and does not synthesize missing data points.
- Remove the data-zoom slider from the intraday view.

## Quantitative Overlays

Only charts opened from a quant target whose strategy is `intraday_t` receive overlays. `KlineChart` accepts an optional `intradayWindows` prop containing `sellWindows` and `buybackWindows`; the holdings modal omits it. Quant Alerts replaces its holding-only modal state with a chart-input object containing the derived holding and, for intraday-T targets, those two window arrays.

- `intraday_high_frequency_windows` render as a warm, translucent band labeled `卖T关注`.
- `intraday_low_frequency_windows` render as a cool, translucent band labeled `买回关注`.
- A window includes bars from its start time through its end time. A bar at `11:30` belongs to the morning window and one at `13:00` belongs to the afternoon window; no band spans the lunch break.
- If both kinds of windows contain a bar, render both translucent bands and place their labels at separate vertical offsets.
- The first version shows attention windows only. It does not add individual notification or execution markers.
- Ordinary holdings charts do not render quantitative bands, even when they have an associated quant target.

## Architecture

`KlineChart` remains the chart owner. It receives optional, presentation-only intraday-window data when opened from a quant target.

The existing `api.getKlineData(code, kind, "5m", limit)` request and `KlineBar` structure remain unchanged. For the intraday request, the component requests `120` bars, which covers two complete A-share 5-minute sessions (96 bars) plus provider boundary variation. It accepts provider-local `YYYY-MM-DD HH:mm` or `YYYY-MM-DD HH:mm:ss` timestamps as `Asia/Shanghai` wall-clock times without using browser-local `Date` conversion, discards other timestamp formats, sorts the remaining bars by timestamp, then filters to the newest date. The line uses each selected bar's `close` and is allowed to be partial while that date is trading; it is not treated as an error.

The prior close is the final valid `close > 0` bar on the immediate preceding returned date. If it is unavailable, the chart renders the price line and volume but omits the previous-close reference and percentage axis. With a prior close, let `d` be the larger of the largest absolute difference between the previous close and any displayed low/high or `previousClose * 0.01`. The main price range is `[previousClose - d, previousClose + d]`; the percentage range is `[-d / previousClose * 100, d / previousClose * 100]`. The right axis maps a price `p` to `(p / previousClose - 1) * 100`, formatted to two decimal places; therefore `0.00%` exactly aligns with the previous-close line.

No quote-provider, Tauri command, database, or market-depth API changes are required.

## Interaction And Errors

- Daily, weekly, and monthly stock charts retain the existing candlestick behavior.
- Fund charts retain their daily NAV line behavior.
- Existing cancellation, loading, and empty-state handling remains in place.
- Before the market opens, during a holiday, or when today's 5-minute data is absent, the newest date returned by the provider is shown. This includes a partial provider-delayed day when it is the newest returned date.
- A time window that has no matching intraday bars is ignored.

## Verification

- Unit-test selecting one latest trading day, deriving the prior close, price/percentage axis conversion, volume colors, and mapping quant windows to chart bands.
- Verify standard holdings have no quant overlays and intraday-T targets have the returned attention bands.
- Run `npm run typecheck` and `npm run build`.
- Run the existing 5-minute stock-kline backend test.
