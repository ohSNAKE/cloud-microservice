# Quant Card K-Line Entry Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a K-line chart icon to Quant Alerts cards and open the existing K-line modal from the card.

**Architecture:** Reuse the existing `KlineChart` component and Ant Design `Modal`. Add local state to `QuantAlerts.tsx` for the selected target converted into a temporary `Holding`-shaped object.

**Tech Stack:** React, Ant Design, TypeScript, existing `KlineChart` component.

---

## Chunk 1: K-Line Entry On Quant Cards

### Task 1: Add Header Icon And Modal

**Files:**
- Modify: `src/pages/QuantAlerts.tsx`

- [ ] Import `Modal`, `Tooltip`, `LineChartOutlined`, `KlineChart`, and `Holding` as needed.
- [ ] Add `klineHolding` state.
- [ ] Add a helper that maps `QuantTarget` to a temporary `Holding` object for chart rendering.
- [ ] Add a small text button with `LineChartOutlined` in `.quant-target-card__header` next to the status tag.
- [ ] Add an Ant Design modal at page bottom rendering `<KlineChart holding={klineHolding} />`.
- [ ] Run `npm run typecheck` and `npm run build`.
