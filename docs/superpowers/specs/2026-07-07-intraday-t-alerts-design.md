# Intraday T Alerts Design

## Context

The app already has a Tauri 2 + React + TypeScript + SQLite quant alert module. The existing module supports a `auto_grid` strategy that uses MA5 / MA20 trend state, automatic grid zones, strict realtime quote validation, a 5-minute cooldown, signal history, in-app display, and desktop notifications.

This design adds a second strategy mode named `intraday_t`. It is for A-share intraday T+0-style decision support, especially weak stocks where the practical pattern is often sell first during historically strong intraday windows and buy back later during historically weak windows.

The feature remains an analysis and reminder tool only. It does not place trades, calculate position size, mutate holdings, or create accounting transactions.

## Goals

- Add a per-target strategy mode switch: `auto_grid` or `intraday_t`.
- Detect high-frequency intraday high/low time windows from recent 5-minute K-line history.
- Trigger `sell_t_attention` when the current time is in a historically high-frequency high window and the current price is high in today's range.
- Trigger `buyback_attention` when the current time is in a historically high-frequency low window and the current price is low in today's range.
- Show the action and reason together in the UI and notifications.
- Reuse the existing strict realtime quote validation, signal persistence, history display, and cooldown model.

## Non-Goals

- No order execution.
- No position sizing.
- No guarantee that high/low windows will repeat.
- No replacement of the existing `auto_grid` strategy.
- No intraday T strategy for funds or non-CN markets in the first version.
- No exchange holiday calendar beyond the current strict realtime validation.

## Strategy Modes

Each quant target has one active strategy mode.

- `auto_grid`: existing MA5 / MA20 plus automatic grid strategy.
- `intraday_t`: new 5-minute K-line time-window strategy.

The default mode for existing and new settings is `auto_grid`, preserving the current behavior unless the user explicitly switches a target to做T mode.

`auto_grid` and `intraday_t` must not emit signals for the same target during the same refresh. The backend selects the active strategy from `quant_strategy_settings.strategy_mode` and evaluates only that strategy.

## Intraday T Rules

### Historical Window Calculation

The backend fetches recent completed 5-minute K-line trading days for the target. The first-version default lookback is 22 completed trading days, excluding the current in-progress trading session from the historical frequency sample.

The service groups 5-minute bars by trading day. For each valid trading day:

- Find the bar where the day high occurs.
- Find the bar where the day low occurs.
- Assign each high and low occurrence to one fixed time bucket.

Time buckets:

- `09:30-09:45`
- `09:50-10:30`
- `10:35-11:30`
- `13:00-13:30`
- `13:35-14:30`
- `14:35-15:00`

High-frequency windows:

- A sell-T window is active when daily highs occurred in that bucket at least `intraday_high_time_min_count` times in the lookback sample.
- A buyback window is active when daily lows occurred in that bucket at least `intraday_low_time_min_count` times in the lookback sample.

Default thresholds:

- `intraday_lookback_days = 22`
- `intraday_high_time_min_count = 4`
- `intraday_low_time_min_count = 4`

If multiple buckets pass a threshold, all qualifying buckets are valid trigger windows.

### Current Day Price Position

The strategy uses today's valid realtime or 5-minute intraday range to confirm price location.

Definition:

`position = (current_price - today_low) / (today_high - today_low)`

Defaults:

- Sell-T threshold: `sell_t_position_threshold = 0.70`
- Buyback threshold: `buyback_position_threshold = 0.30`

### Signal Decisions

For each refresh of an `intraday_t` target:

- If quote retrieval is not strictly realtime, emit `quote_error` and do not persist or notify.
- If there are fewer than 10 valid historical trading days, emit `watch` with reason `分时统计数据不足`.
- If today's high/low range is too narrow to compute a meaningful position, emit `watch` with reason `当日波动不足，暂不触发`.
- If current market time is inside a qualifying high-frequency high bucket and `position >= sell_t_position_threshold`, emit `sell_t_attention`.
- If current market time is inside a qualifying high-frequency low bucket and `position <= buyback_position_threshold`, emit `buyback_attention`.
- Otherwise emit `watch`.

If both sell and buyback conditions somehow pass in the same refresh, the backend should prefer `watch` and include a defensive reason rather than emitting contradictory advice. This should be rare because the price-position thresholds are non-overlapping by default.

MA5 / MA20 trend state is still calculated and displayed as risk context, but `intraday_t` does not use the trend state as a hard filter. This keeps the mode useful for weak stocks where the intended pattern is often early sell and later buyback.

## Data Flow

`refresh_quant_signals()` remains the single backend-owned refresh path.

For each target:

1. Resolve the target and settings.
2. Fetch and validate realtime quote data using the existing strict current-session validation.
3. Calculate trend state for display, as the existing module already does.
4. If `strategy_mode = auto_grid`, run the existing auto-grid logic.
5. If `strategy_mode = intraday_t`, fetch recent completed historical 5-minute K-line days plus the current-day intraday range and run the intraday T logic.
6. Persist only newly generated signals that pass cooldown.
7. Return updated dashboard state and `generated_signals` for desktop notifications.

The frontend must not calculate high-frequency windows or price-position triggers. It only renders backend-returned fields.

## Data Model

### `quant_strategy_settings`

Extend the table with new fields:

- `strategy_mode TEXT NOT NULL DEFAULT 'auto_grid'`
- `intraday_lookback_days INTEGER NOT NULL DEFAULT 22`
- `intraday_high_time_min_count INTEGER NOT NULL DEFAULT 4`
- `intraday_low_time_min_count INTEGER NOT NULL DEFAULT 4`
- `sell_t_position_threshold REAL NOT NULL DEFAULT 0.70`
- `buyback_position_threshold REAL NOT NULL DEFAULT 0.30`

Valid values:

- `strategy_mode`: `auto_grid`, `intraday_t`

The first UI version only needs to expose the strategy mode switch. The numeric parameters can remain backend defaults unless a later settings UI is added.

### `quant_signals`

Reuse the existing signal table.

Extend valid values:

- `direction`: add `sell_t_attention`, `buyback_attention`
- `source`: add `intraday_t`
- `trigger_zone`: for intraday T signals, store the matched time bucket such as `09:30-09:45` or `13:35-14:30`

The dedupe key must include `code`, `market`, `direction`, and `trigger_zone`. This keeps sell-T and buyback reminders independent and preserves the existing 5-minute cooldown behavior.

### API Types

Extend `QuantTarget` with fields needed for display:

- `strategy_mode: "auto_grid" | "intraday_t"`
- `output_state`: add `sell_t_attention` and `buyback_attention`
- `current_trigger_zone`: allow the six intraday time buckets in addition to existing grid zones
- `intraday_position: number | null`
- `intraday_reason: string | null`
- `intraday_high_frequency_windows: string[]`
- `intraday_low_frequency_windows: string[]`

Extend `QuantSignal`:

- `direction`: add `sell_t_attention` and `buyback_attention`
- `source`: allow `auto_grid` or `intraday_t`
- `trigger_zone`: allow grid zones or intraday time buckets

Extend `QuantStrategySettingsUpdate` so the frontend can save `strategy_mode` with existing enabled and desktop-notification settings.

## Frontend Design

The `量化提醒` page keeps its current structure and adds strategy-specific display.

### Target Card

Each target card adds a strategy selector:

- `自动网格`
- `做T`

Changing the selector updates `quant_strategy_settings.strategy_mode` for that target. The change affects subsequent refreshes.

### Intraday T Status

When a target is in做T mode, the card shows:

- Current price.
- Today's range position as a percentage, when available.
- Trend state as risk context.
- High-frequency sell-T windows.
- High-frequency buyback windows.
- Current status and reason.

Status labels:

- `卖T关注`
- `买回关注`
- `观望`
- `行情异常`

Example reason text:

- `卖T关注 · 历史早盘高点高发`
- `买回关注 · 午后低点高发`
- `分时统计数据不足`
- `当日波动不足，暂不触发`

### Signal History

Signal history supports the new directions and source:

- Direction labels: `卖T关注`, `买回关注`
- Source label: `做T`
- Trigger zone: display the time bucket.

## Notifications

Desktop notifications continue to be sent only for `generated_signals` returned by `refresh_quant_signals()`.

Examples:

- Title: `量化提醒：世嘉科技 卖T关注`
- Body: `历史高发时段 09:30-09:45，当前日内位置 76%，仅供参考。`
- Title: `量化提醒：世嘉科技 买回关注`
- Body: `历史高发时段 13:35-14:30，当前日内位置 24%，仅供参考。`

If desktop notification permission is unavailable or denied, the app still persists signals and shows them in the UI.

## Error Handling And Boundaries

- Data insufficiency: fewer than 10 valid historical trading days does not trigger signals and shows `分时统计数据不足`.
- Narrow current-day range: if `today_high - today_low` is too small for a meaningful percentage, do not trigger and show `当日波动不足，暂不触发`.
- Non-trading time: do not create new reminders; the page may show the last computed statistics and history.
- Non-realtime quote: use the existing strict realtime validation and do not persist or notify.
- Quote/K-line failures: affect only the failed target.
- Weak trend: do not block intraday T signals, but continue showing trend state as risk context.
- Cooldown: same stock, market, direction, and time bucket can notify at most once every 5 minutes.

The implementation should choose a small positive epsilon for the narrow-range check so division is safe and meaningless flat-range signals are avoided.

## Testing

### Backend Unit Tests

Cover:

- 5-minute K-line grouping by trading day.
- Daily high and low occurrence detection.
- Time-bucket assignment.
- High-frequency high-window threshold detection.
- High-frequency low-window threshold detection.
- Current-day range-position calculation.
- Sell-T signal generation from time window plus `position >= 0.70`.
- Buyback signal generation from time window plus `position <= 0.30`.
- Insufficient historical data does not trigger.
- Narrow current-day range does not trigger.
- Non-trading time does not create new intraday T signals.
- Stale or non-realtime quotes produce `quote_error` and do not persist or notify.
- Weak trend state remains visible but does not block otherwise valid intraday T signals.
- Contradictory sell/buyback conditions do not emit contradictory signals.
- Intraday T signals persist with `source = intraday_t` and time-bucket `trigger_zone`.
- 5-minute cooldown dedupes by stock, market, direction, and time bucket.
- `auto_grid` and `intraday_t` strategy modes do not affect each other.

### Backend Command Tests

Cover:

- Updating `strategy_mode` persists per target.
- `refresh_quant_signals()` evaluates only the selected strategy mode.
- Dashboard responses include intraday T display fields for做T targets.
- Existing auto-grid targets keep previous behavior by default.

### Frontend Verification

Run:

- `npm run typecheck`
- `npm run build`

Manual checks:

- Switch a target between `自动网格` and `做T` and confirm the setting is saved.
- Confirm做T status, reason, and day-position percentage render correctly.
- Confirm signal history displays `卖T关注`, `买回关注`, and source `做T`.
- Confirm desktop notification titles and bodies use the new action labels.

## Acceptance Criteria

- Users can choose `自动网格` or `做T` per quant target.
- Existing targets default to `自动网格` and keep current behavior.
- 做T mode analyzes recent 5-minute K-line high/low time windows.
- 做T mode combines historical time-window frequency with current-day price position before triggering.
- `卖T关注` and `买回关注` are persisted, shown in history, and eligible for desktop notification.
- Signals are not persisted or notified when strict realtime quote validation fails.
- Data insufficiency, flat intraday range, and non-trading periods do not create false reminders.
- The feature remains read-only and does not mutate holdings, accounts, transactions, or order state.
