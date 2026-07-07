# Quant Trading Alerts Design

## Context

The app is a Tauri 2 + React + TypeScript + SQLite desktop finance app. It already supports local accounts, transactions, investment holdings, quote refresh, and K-line display for stocks and funds.

This design adds a first version of a quantitative trading alert module. The module is an analysis and reminder tool only. It does not place orders, recommend share quantities, modify holdings, or create accounting transactions.

## Goals

- Provide intraday stock reminders for short-term trading decisions.
- Support both existing stock holdings and a separate quant watchlist.
- Use MA5 / MA20 to judge daily trend direction.
- Use automatically calculated grid levels to trigger intraday buy/sell attention signals.
- Show signals inside the app and trigger desktop notifications.
- Persist signal history locally for review.
- Prevent notification spam with a 5-minute cooldown per stock, direction, and trigger zone.

## Non-Goals

- No real brokerage integration.
- No automatic order placement.
- No suggested trade quantity.
- No modification of existing holdings, account balances, or transaction records.
- No promise of prediction accuracy.
- No fund support for intraday trading alerts in the first version.
- No exchange holiday calendar in the first version.

## Architecture

The module is split into three bounded areas.

### Target Pool

Targets come from two sources:

- Existing stock holdings from `holdings` where `type = 'stock'`.
- Quant watchlist entries created inside the new module.

Funds are excluded from intraday alerts because they do not provide useful minute-level trading feedback for this use case.

Each target has an enabled state. Held stocks are automatically available for analysis, while watchlist stocks let the user observe names that are not currently held.

### Signal Engine

The signal engine combines daily trend filtering with intraday grid triggers.

- Daily layer: fetch recent completed daily K-line data and calculate MA5 and MA20.
- Trend state: classify each stock as bullish, bearish, or neutral using the rules in the trend section below.
- Grid layer: automatically generate buy and sell trigger zones using the formula in the grid section below.
- Intraday layer: poll current or minute-level price during A-share trading hours.
- Output state: `buy_attention`, `sell_attention`, `watch`, or `quote_error`.

The first version emits direction-only advice: buy attention or sell attention. It does not include quantity sizing.

### Trend Rules

Trend calculation uses the latest completed daily K-line close values.

For intraday polling, MA and grid calculations must exclude an unfinished daily bar for the current `Asia/Shanghai` date. If the quote provider returns a daily bar for the current date before that trading day has closed, exclude it from MA and grid calculations. Outside trading hours after `15:00`, the current day's daily bar may be used for display and next-session preparation, but it must not retroactively change signals already emitted earlier in the day.

- `MA5` is the arithmetic average of the latest 5 closes.
- `MA20` is the arithmetic average of the latest 20 closes.
- Trend is `bullish` when `MA5 > MA20` and the latest close is greater than or equal to `MA5`.
- Trend is `bearish` when `MA5 < MA20` and the latest close is less than or equal to `MA5`.
- Trend is `neutral` in all other cases.
- If fewer than 20 daily bars are available, trend is `insufficient_data` and no buy/sell attention signal is emitted for that target.

In signal rules, "not clearly bearish" means trend is `bullish` or `neutral`. "Not clearly bullish" means trend is `bearish` or `neutral`.

### Automatic Grid Rules

Grid generation uses the latest `grid_lookback_days` daily K-line bars. The first-version default is 20 days.

Definitions:

- `base_price` is the latest daily close.
- `lookback_high` is the highest high in the lookback window.
- `lookback_low` is the lowest low in the lookback window.
- `range_step = (lookback_high - lookback_low) / 6`.
- `avg_abs_move` is the average absolute close-to-close move across the lookback window. For `N` closes ordered oldest to newest, use `abs(close[i] - close[i - 1])` for `i = 1..N-1` and divide by `N - 1`.
- `min_step = base_price * 0.003`.
- `grid_step = max(range_step, avg_abs_move, min_step)`.

Grid generation requires at least `grid_lookback_days` bars and at least two close values. If there is not enough data, the target keeps `trend_state = insufficient_data`, has no grid zones, and does not notify.

Generated zones:

- `buy_1`: `[base_price - grid_step, base_price)`.
- `buy_2`: `[base_price - 2 * grid_step, base_price - grid_step)`.
- `sell_1`: `(base_price, base_price + grid_step]`.
- `sell_2`: `(base_price + grid_step, base_price + 2 * grid_step]`.

The `trigger_zone` stored in `quant_signals` is one of `buy_1`, `buy_2`, `sell_1`, or `sell_2`.

Signal direction from zones:

- Price inside `buy_1` or `buy_2` can emit `buy_attention` if trend is not `bearish`.
- Price inside `sell_1` or `sell_2` can emit `sell_attention` if trend is not `bullish`.
- If price is outside all zones, emit `watch`.
- If trend is `insufficient_data`, emit `watch` with a reason and do not notify.

### Reminder And History

When a candidate signal passes cooldown checks, the app writes it to SQLite, updates the in-app signal list, and triggers a desktop notification.

Signals remain visible for review even after the price moves away from the trigger zone.

## Data Flow

### Initialization

When the quant page opens, the frontend starts one polling session for the page. The first version keeps polling trigger ownership in the frontend to avoid a background engine that runs when the user has not opened the feature.

The page requests from backend commands:

- Existing stock holdings.
- Enabled watchlist stocks.
- Strategy settings.
- Recent signal history.

For each enabled target, the backend fetches recent completed daily K-line data and computes trend state and automatic grid zones. The frontend must not compute or cache trend/grid state.

The frontend must prevent duplicate polling by keeping a single active interval per mounted quant page. The interval stops when the page unmounts. Manual refresh reuses the same refresh function but must not create another interval.

### Intraday Polling

During A-share trading hours, the module polls current or minute-level prices at a fixed interval.

The first version treats trading hours as:

- `09:30-11:30`
- `13:00-15:00`

Times are evaluated in local China market time, `Asia/Shanghai`. The first version treats Monday through Friday as possible trading days and does not trigger desktop alerts on Saturday or Sunday.

It does not include a holiday calendar. Instead, strict realtime validation is required. Each intraday signal decision must require a successful quote fetch that includes an exchange quote timestamp or explicit market-status field confirming the quote belongs to the current `Asia/Shanghai` trading session. If the quote API fails, returns no usable price, lacks an exchange timestamp or market-status field, or reports a timestamp outside the active trading session, the target becomes `quote_error` and no new signal is persisted or notified.

If the market is closed, the page keeps showing the latest trend and recent signals, but desktop alerts are not triggered.

Quant intraday quote fetching must be read-only. It must not call the existing `refresh_quotes` command because that command mutates `holdings` and `price_history`. The quant service should provide a separate read-only realtime quote fetch path that returns price, exchange quote timestamp or market status, and `quote_fetched_at` without writing to existing finance tables. If the selected provider cannot return exchange time or market status, it is not acceptable for strict mode alerts.

### Signal Generation

For each poll:

- If quote retrieval fails, only that stock becomes `quote_error`.
- If price enters a buy grid zone and the daily trend is not clearly bearish, emit `buy_attention`.
- If price enters a sell grid zone and the daily trend is not clearly bullish, emit `sell_attention`.
- Otherwise emit `watch`.

Candidate signals are checked against the cooldown rule before persistence and notification.

Trend and grid state are backend-owned. `refresh_quant_signals()` recomputes target list, completed daily K-line trend, automatic grid zones, realtime quote validation, current output state, and newly generated signals on each call. The frontend does not calculate or cache trend/grid state across polls; it only renders the dashboard returned by the backend. This keeps polling stateless outside SQLite and avoids frontend/backend divergence.

### Cooldown

The cooldown key is based on:

- Stock code.
- Market.
- Direction.
- Trigger zone.

The cooldown duration is 5 minutes. The same stock, market, direction, and zone can trigger another in-app and desktop reminder after 5 minutes if the price is still inside the trigger zone. The `dedupe_key` must include `code`, `market`, `direction`, and `trigger_zone`, or an equivalent unambiguous encoding of those fields.

Different stocks and different directions do not share cooldown state.

## Data Model

### `quant_watchlist`

Stores user-managed non-holding stocks for quant analysis.

Fields:

- `id INTEGER PRIMARY KEY AUTOINCREMENT`
- `code TEXT NOT NULL`
- `name TEXT NOT NULL`
- `market TEXT NOT NULL DEFAULT 'cn'`
- `enabled INTEGER NOT NULL DEFAULT 1`
- `created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))`
- `updated_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))`

Recommended constraint:

- `UNIQUE(code, market)`

### `quant_strategy_settings`

Stores per-target strategy settings. Defaults can be inserted lazily when a target first appears.

When lazily inserting settings, initial values must be derived from the effective target source. For holding targets, `enabled` defaults to `1`. For watchlist-only targets, `enabled` must copy `quant_watchlist.enabled` instead of blindly using the table default. This prevents a disabled watchlist row from becoming enabled when a settings row is created.

Fields:

- `id INTEGER PRIMARY KEY AUTOINCREMENT`
- `code TEXT NOT NULL`
- `market TEXT NOT NULL DEFAULT 'cn'`
- `ma_short INTEGER NOT NULL DEFAULT 5`
- `ma_long INTEGER NOT NULL DEFAULT 20`
- `grid_lookback_days INTEGER NOT NULL DEFAULT 20`
- `poll_interval_seconds INTEGER NOT NULL DEFAULT 60`
- `desktop_notification_enabled INTEGER NOT NULL DEFAULT 1`
- `enabled INTEGER NOT NULL DEFAULT 1`
- `created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))`
- `updated_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))`

Recommended constraint:

- `UNIQUE(code, market)`

The UI in the first version should not expose every parameter. It should prioritize target management and enable/disable controls.

`enabled` controls whether the target participates in polling and signal generation. `desktop_notification_enabled` controls desktop notification delivery only. When `desktop_notification_enabled = 0`, signals are still evaluated, persisted after cooldown, and shown in the app, but no desktop notification is sent for that target.

### `quant_signals`

Stores triggered signal history.

Fields:

- `id INTEGER PRIMARY KEY AUTOINCREMENT`
- `code TEXT NOT NULL`
- `name TEXT NOT NULL`
- `market TEXT NOT NULL DEFAULT 'cn'`
- `direction TEXT NOT NULL`
- `trigger_price REAL NOT NULL`
- `trend_state TEXT NOT NULL`
- `source TEXT NOT NULL`
- `trigger_zone TEXT NOT NULL`
- `dedupe_key TEXT NOT NULL`
- `triggered_at TEXT NOT NULL`
- `created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))`

Useful indexes:

- `idx_quant_signals_code_time ON quant_signals(code, triggered_at)`
- `idx_quant_signals_dedupe_time ON quant_signals(dedupe_key, triggered_at)`

## Target Identity And Deduplication

Quant target identity is `(code, market)`.

Existing holdings may contain multiple rows for the same stock. The quant module collapses them into one target by `(code, market)` and uses the latest non-empty name found in holdings. If a watchlist entry has the same `(code, market)` as an existing holding, the combined target is marked as both `holding` and `watchlist`, but appears once in the signal board.

Effective enabled state is resolved in this order:

- If `quant_strategy_settings` has a row for `(code, market)`, use `quant_strategy_settings.enabled`.
- If no settings row exists and the target has a holding source, default to enabled.
- If no settings row exists and the target is watchlist-only, use `quant_watchlist.enabled`.

The signal board shows a single enable/disable control per combined target. Toggling it always creates or updates `quant_strategy_settings.enabled` for `(code, market)`. It never modifies `holdings`. For watchlist-only targets, `quant_watchlist.enabled` is used only as the initial default before a settings row exists.

Watchlist management may show whether a watchlist row is initially enabled, but after a settings row exists the signal board's `quant_strategy_settings.enabled` value is authoritative. Deleting a watchlist row removes that watchlist source only; if the same `(code, market)` is also held, the combined holding target remains available and keeps its strategy settings.

## Backend Commands

The frontend should access the module through Tauri commands, consistent with the existing API pattern.

Proposed commands:

- `list_quant_watchlist()`
- `add_quant_watchlist(input)`
- `update_quant_watchlist(id, input)`
- `delete_quant_watchlist(id)`
- `update_quant_strategy_settings(code, market, input)`
- `list_quant_targets()`
- `get_quant_dashboard()`
- `refresh_quant_signals()`
- `list_quant_signals(filter)`

The implementation can keep signal calculations in a dedicated quant service module to avoid expanding holdings command responsibilities.

### API Contracts

`QuantWatchlistItem`:

- `id: number`
- `code: string`
- `name: string`
- `market: string`
- `enabled: boolean`
- `created_at: string`
- `updated_at: string`

`NewQuantWatchlistItem`:

- `code: string`
- `name?: string`
- `market?: string`
- `enabled?: boolean`

`QuantTarget`:

- `code: string`
- `name: string`
- `market: string`
- `source: "holding" | "watchlist" | "holding_watchlist"`
- `enabled: boolean`
- `desktop_notification_enabled: boolean`
- `current_price: number | null`
- `quote_fetched_at: string | null`
- `trend_state: "bullish" | "bearish" | "neutral" | "insufficient_data"`
- `output_state: "buy_attention" | "sell_attention" | "watch" | "quote_error"`
- `current_trigger_zone: "buy_1" | "buy_2" | "sell_1" | "sell_2" | null`
- `ma_short: number | null`
- `ma_long: number | null`
- `grid_zones: QuantGridZone[]`
- `latest_signal: QuantSignal | null`
- `last_error: string | null`

`output_state` is the current poll result and drives the signal board. `latest_signal` is historical and must not be used to infer the current state after price leaves a trigger zone. Quote failure is represented by `output_state = "quote_error"` and `last_error`; it is not a trend state.

`QuantGridZone`:

- `zone: "buy_1" | "buy_2" | "sell_1" | "sell_2"`
- `direction: "buy_attention" | "sell_attention"`
- `lower: number`
- `upper: number`

`QuantSignal`:

- `id: number`
- `code: string`
- `name: string`
- `market: string`
- `direction: "buy_attention" | "sell_attention"`
- `trigger_price: number`
- `trend_state: "bullish" | "bearish" | "neutral"`
- `source: "auto_grid"`
- `trigger_zone: "buy_1" | "buy_2" | "sell_1" | "sell_2"`
- `triggered_at: string`

`QuantGeneratedSignal`:

- `signal: QuantSignal`
- `desktop_notification_enabled: boolean`

The frontend sends desktop notifications only when `desktop_notification_enabled` is true.

`QuantDashboard`:

- `is_trading_time: boolean`
- `next_refresh_at: string | null`
- `targets: QuantTarget[]`
- `recent_signals: QuantSignal[]`

`QuantRefreshResult`:

- `dashboard: QuantDashboard`
- `generated_signals: QuantGeneratedSignal[]`

`generated_signals` contains only signals newly persisted during the current refresh call. The frontend uses this list for desktop notifications so it does not replay older signals.

`QuantSignalFilter`:

- `code?: string`
- `direction?: "buy_attention" | "sell_attention"`
- `from?: string`
- `to?: string`
- `limit?: number`

`refresh_quant_signals()` returns `QuantRefreshResult` so the frontend can update the board and notify only for newly generated signals.

Command return mapping:

- `list_quant_watchlist() -> QuantWatchlistItem[]`
- `add_quant_watchlist(input: NewQuantWatchlistItem) -> QuantWatchlistItem`
- `update_quant_watchlist(id: number, input: QuantWatchlistItemUpdate) -> QuantWatchlistItem`
- `delete_quant_watchlist(id: number) -> void`
- `update_quant_strategy_settings(code: string, market: string, input: QuantStrategySettingsUpdate) -> QuantTarget`
- `list_quant_targets() -> QuantTarget[]`
- `get_quant_dashboard() -> QuantDashboard`
- `refresh_quant_signals() -> QuantRefreshResult`
- `list_quant_signals(filter?: QuantSignalFilter) -> QuantSignal[]`

`QuantWatchlistItemUpdate`:

- `name: string`
- `enabled: boolean`

`QuantStrategySettingsUpdate`:

- `enabled: boolean`
- `desktop_notification_enabled: boolean`

## Frontend Design

Add a left menu item named `量化提醒`.

The page contains four sections.

### Top Status

Shows:

- Whether current time is inside trading hours.
- Polling state.
- Next refresh time.
- A visible risk disclaimer: `信号仅供参考，不构成投资建议`.

### Signal Board

Shows each target's latest state:

- Stock name and code.
- Current price.
- Trend state.
- Latest signal direction.
- Latest trigger time.
- Enable/disable control.

Statuses:

- `买入关注`
- `卖出关注`
- `观望`
- `行情异常`

### Watchlist Management

Allows the user to:

- Add non-holding stocks.
- Resolve stock names through the existing quote lookup pattern.
- Delete watchlist stocks.
- Set the watchlist row's initial enabled state before a per-target strategy settings row exists.

Existing held stocks appear automatically and do not need to be duplicated in the watchlist.

### Signal History

Shows recent triggered signals with filters by stock and direction.

Each row contains:

- Time.
- Stock.
- Direction.
- Trigger price.
- Source.

## Notifications

The module should trigger both in-app feedback and desktop notifications when a signal passes cooldown.

Notification text should be concise and avoid certainty.

Example:

- `量化提醒：贵州茅台 触发买入关注`
- `当前价格进入自动网格买入区，仅供参考。`

If desktop notification permission is unavailable, the app should still record the signal and show it in the UI.

The implementation must add Tauri desktop notification support with the Tauri 2 notification plugin:

- Rust dependency: add `tauri-plugin-notification = "2"` to `src-tauri/Cargo.toml`.
- Frontend dependency: add `@tauri-apps/plugin-notification` to `package.json`.
- Rust initialization: call `.plugin(tauri_plugin_notification::init())` in the Tauri builder.
- Capability permissions: add notification permission to the app capability, preferably `notification:default`; if explicit generated permissions are required, include permission for checking permission, requesting permission, and sending notifications.
- Frontend API: use the plugin API to check notification permission, request permission on first use, and send notifications only when permission is granted.

If permission is denied, the UI should show an unobtrusive warning and continue with in-app reminders.

The frontend must send desktop notifications only for `generated_signals` returned by `refresh_quant_signals()`. It must not notify from `recent_signals` or `latest_signal`, because those may contain historical records.

## Backup And Import

The existing export/import feature must include the new quant tables:

- `quant_watchlist`
- `quant_strategy_settings`
- `quant_signals`

Export should preserve watchlist, settings, and signal history. Import should restore these tables together with the rest of the local finance database. This avoids losing user-managed watchlists and prevents orphaned quant settings after a restore.

Import must remain compatible with backups created before the quant module existed. Extend the existing `ExportPayload` with these optional arrays using `#[serde(default)]` so old JSON files deserialize successfully:

- `quant_watchlist: QuantWatchlistRow[]`
- `quant_strategy_settings: QuantStrategySettingsRow[]`
- `quant_signals: QuantSignalRow[]`

If a backup payload has no quant fields, treat them as empty arrays. During destructive restore, create/migrate the quant tables first, then clear them together with the rest of the restored database tables before inserting imported rows. The cleanup order should delete `quant_signals`, then `quant_strategy_settings`, then `quant_watchlist` before inserting restored quant rows. This prevents stale quant rows from the previous local database when importing old backups.

Export row payloads should map directly to table columns:

- `QuantWatchlistRow`: `id`, `code`, `name`, `market`, `enabled`, `created_at`, `updated_at`.
- `QuantStrategySettingsRow`: `id`, `code`, `market`, `ma_short`, `ma_long`, `grid_lookback_days`, `poll_interval_seconds`, `desktop_notification_enabled`, `enabled`, `created_at`, `updated_at`.
- `QuantSignalRow`: `id`, `code`, `name`, `market`, `direction`, `trigger_price`, `trend_state`, `source`, `trigger_zone`, `dedupe_key`, `triggered_at`, `created_at`.

## Error Handling

- Quote failure affects only the failed stock.
- Failed stocks show `行情异常` and keep their last successful signal history.
- Other targets continue polling.
- Non-trading hours do not trigger desktop reminders.
- Missing or insufficient K-line data should mark the target as unable to calculate trend instead of crashing the page.
- Duplicate watchlist additions should return a user-friendly message.

## Testing

### Backend Unit Tests

Cover:

- MA calculation.
- Trend classification.
- Automatic grid zone generation.
- Buy/sell/watch signal decisions.
- 5-minute cooldown behavior using `code + market + direction + trigger_zone` identity.
- Strict realtime quote validation rejects missing exchange timestamp or market status.

### Backend Command Tests

Cover:

- Add, update, and delete watchlist entries.
- List combined quant targets from holdings and watchlist.
- Persist and query signal history.
- Per-target quote failure isolation.
- `refresh_quant_signals()` recomputes backend-owned trend/grid/current state and returns only newly generated signals in `generated_signals`.
- Export/import handles quant tables and imports pre-quant backups with missing quant arrays.

### Frontend Verification

Run:

- `npm run typecheck`
- `npm run build`

Manual checks:

- Add a watchlist stock.
- Enable and disable reminders.
- Confirm current status and trend render correctly.
- Confirm signal history renders.
- Confirm repeated reminders respect the 5-minute cooldown.
- Confirm denied desktop notification permission falls back to in-app signal records without crashing.

## Acceptance Criteria

- A `量化提醒` page exists in the app navigation.
- Existing stock holdings and watchlist stocks appear as quant targets.
- The module calculates MA5 / MA20 trend state for each target with enough data.
- The module generates automatic grid-based buy/sell attention signals.
- Signals are direction-only and do not include trade quantity.
- Triggered signals are saved locally.
- In-app and desktop notifications are emitted for signals that pass cooldown.
- The same stock, market, direction, and trigger zone can notify at most once every 5 minutes.
- Signals are not persisted or notified when strict realtime quote validation fails.
- Quote failures do not break the whole page or stop other stocks.
- Trend, grid, and current output state are recomputed by the backend on each refresh call.
- Desktop notification permission denial still allows in-app signal records.
- Existing backups without quant fields import successfully and clear stale local quant rows.
- The module does not execute trades or mutate existing holdings, accounts, or transactions.
