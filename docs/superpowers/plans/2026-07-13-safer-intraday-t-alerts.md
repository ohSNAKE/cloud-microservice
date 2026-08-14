# Safer Intraday T Alerts Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the intraday T strategy safer for volatile, weak A-shares by separating an opening watch from a confirmed sell-T signal and requiring an explicitly recorded sell plus a reversal before a buyback signal.

**Architecture:** Keep the detection rules as pure helpers in `services/quant.rs`. Persist only the user's same-day sell-T acknowledgement in a small SQLite table; the dashboard reads that state and the buyback rule receives it as an explicit input. The React client renders a lightweight opening warning and lets the user record or clear the same-day sell without attempting to record an actual broker trade.

**Tech Stack:** Tauri 2, Rust, rusqlite, chrono, React 19, TypeScript, Ant Design.

---

## File Structure

- Modify `src-tauri/src/db.rs`: create and migrate `quant_intraday_t_state`.
- Modify `src-tauri/src/models.rs`: expose the per-target same-day T-state and a request DTO.
- Modify `src-tauri/src/services/quant.rs`: add pure opening-watch, sell-confirmation, and buyback-reversal helpers and tests.
- Modify `src-tauri/src/commands/quant.rs`: read/write same-day state, evaluate the guarded intraday rule, and expose T-state commands.
- Modify `src-tauri/src/lib.rs`: register the two new Tauri commands.
- Modify `src-tauri/src/commands/data.rs`: include same-day T-state in backup export/import.
- Modify `src/types/index.ts` and `src/api/index.ts`: type and invoke the new state commands.
- Modify `src/pages/QuantAlerts.tsx`: display opening-watch state and provide explicit `已卖出T仓` / `取消记录` controls.
- Modify `src/utils/quantNotifications.ts`: keep notifications limited to confirmed sell-T and guarded buyback signals.

## Chunk 1: Rule Guards And State

### Task 1: Add failing pure-rule tests

**Files:**
- Modify: `src-tauri/src/services/quant.rs`

- [ ] Add a test proving a high-position setup returns an opening-watch state, not a sell-T signal, only from `09:30:00` through `09:34:59`.
- [ ] Add a test proving a high-frequency early-window setup only returns `sell_t_attention` after `09:35` and after the quote retreats from the current-day high by the confirmation threshold.
- [ ] Add a test proving a low-frequency low-position setup does not return `buyback_attention` without a recorded sell-T state.
- [ ] Add a test proving a recorded sell-T state still requires a rebound from the day low and two non-lower-low completed 5-minute bars; the currently forming bar must never count as either confirmation bar.
- [ ] Add a test proving `14:35-15:00` never returns automatic buyback attention.
- [ ] Run `cargo test --manifest-path src-tauri/Cargo.toml services::quant -- --nocapture` and verify the new tests fail because the helpers/guards do not exist.

### Task 2: Implement minimal pure intraday helpers

**Files:**
- Modify: `src-tauri/src/services/quant.rs`

- [ ] Add constants for the first 5-minute confirmation time, a `0.5%` minimum high-to-current retreat, and a `0.6%` minimum low-to-current rebound; all percentages are relative to the current-day high or low respectively.
- [ ] Add a helper that returns `sell_t_watch` only in `09:30-09:34` when a historical high window and high day position match.
- [ ] Extend `decide_intraday_t_signal` (or use a small input struct if it keeps the API clear) to require a post-`09:35` retreat for `sell_t_attention`.
- [ ] Require `has_sold_t_today`, a rebound of at least `0.6%` from the current-day low, and two completed non-lower-low bars before `buyback_attention`. The current-day low bar must strictly precede both confirmation bars; of those two chronological bars, the later low must be greater than or equal to the earlier low. In `14:35-15:00`, always return the regular `watch` state and suppress every intraday T signal, including `sell_t_watch` and `sell_t_attention`.
- [ ] Treat EastMoney's 5-minute bar timestamp as its close time. Supply only bars whose close timestamp is strictly earlier than `now` to the reversal helper, so a bar closing at the current minute is still treated as forming; add boundary tests for `09:35:00` and `09:36:00`.
- [ ] Run the targeted test command and verify it passes.

### Task 3: Persist same-day sell-T acknowledgement

**Files:**
- Modify: `src-tauri/src/db.rs`
- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/commands/quant.rs`

- [ ] Add command tests that mark a target as sold on a given China-market date, read it on the dashboard, and clear it.
- [ ] Run the targeted command tests and verify they fail because the table and commands do not exist.
- [ ] Add `quant_intraday_t_state(code, market, trading_date, sold_at, PRIMARY KEY(code, market, trading_date))`, including an idempotent migration. Store `sold_at` as a China-time `YYYY-MM-DD HH:MM:SS` value.
- [ ] Add `has_sold_t_today` to `QuantTarget`; read only the current China-market date so stale acknowledgements cannot carry into the next session. Pass `sold_at` to the decision and require it to precede the day-low bar and both reversal-confirmation bars.
- [ ] Add narrow commands to mark and clear the acknowledgement. Validate the target, supported market, and that its strategy is currently `intraday_t`; fetch its quote and require `quote_is_strictly_realtime` at the current China time before changing state. This supplies an exchange-live market-status check rather than relying only on weekday and clock time.
- [ ] Register those commands in `src-tauri/src/lib.rs`.
- [ ] Pass the loaded state, strictly completed current-day minute bars, and a strictly real-time quote into the guarded decision helper. Do not make a T decision from a quote whose exchange timestamp is older than the configured real-time tolerance.
- [ ] Run targeted command tests and verify they pass.

### Task 4: Preserve same-day acknowledgement in backups

**Files:**
- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/commands/data.rs`

- [ ] Add failing export/import round-trip coverage for a same-day sell-T acknowledgement.
- [ ] Extend the backup payload and import transaction to carry `quant_intraday_t_state`; retain backwards compatibility for backups that predate the table.
- [ ] Run the focused data command tests and verify they pass.

## Chunk 2: Dashboard And Notifications

### Task 5: Extend typed client API

**Files:**
- Modify: `src/types/index.ts`
- Modify: `src/api/index.ts`

- [ ] Add `sell_t_watch` as a dashboard-only output state, add `has_sold_t_today` to `QuantTarget`, and type the mark/clear command wrappers.
- [ ] Run `npm run typecheck` and verify it fails until the page handles the additional output state and target field.

### Task 6: Render explicit T-state controls

**Files:**
- Modify: `src/pages/QuantAlerts.tsx`

- [ ] Render `早盘冲高预警` as a non-actionable dashboard status.
- [ ] For `intraday_t` targets, show whether same-day T shares are marked sold and provide `已卖出T仓` / `取消记录` actions.
- [ ] Disable both mark and clear actions outside the valid trading session and refresh the dashboard after either action.
- [ ] State in the UI that the record is only a reminder prerequisite and does not execute or verify a broker order.
- [ ] Run `npm run typecheck` and verify it passes.

### Task 7: Limit desktop notifications to confirmed actions

**Files:**
- Modify: `src-tauri/src/commands/quant.rs`
- Modify: `src/utils/quantNotifications.ts`

- [ ] Add a test proving `sell_t_watch` is not persisted or sent as a generated desktop notification.
- [ ] Verify the test fails before the refresh filter excludes the dashboard-only state.
- [ ] Keep generated notifications limited to `sell_t_attention` and guarded `buyback_attention`; update wording to say the buyback includes a stop-fall confirmation.
- [ ] Run focused Rust tests and `npm run typecheck`.

## Final Verification

- [ ] Run `cargo test --manifest-path src-tauri/Cargo.toml -- --nocapture`.
- [ ] Run `cargo check --manifest-path src-tauri/Cargo.toml`.
- [ ] Run `npm run typecheck`.
- [ ] Run `npm run build`.
- [ ] Inspect `git diff --check` and the diff to ensure no unrelated changes are included.
