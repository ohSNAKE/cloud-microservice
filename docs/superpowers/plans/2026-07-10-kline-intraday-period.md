# K-Line Intraday Period Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `分时` option to the K-line modal by using the existing `5m` backend period.

**Architecture:** This is a frontend-only exposure of an already-supported backend period. The `KlineChart` component keeps its current fetching flow and passes `period` through `api.getKlineData`.

**Tech Stack:** React, TypeScript, Ant Design Radio buttons, existing Tauri command API.

---

## Chunk 1: Add 5m Period To K-Line UI

### Task 1: Extend Period Type And Selector

**Files:**
- Modify: `src/types/index.ts`
- Modify: `src/components/KlineChart.tsx`

- [ ] Extend `KlinePeriod` to include `"5m"`.
- [ ] Add `{ label: "分时", value: "5m" }` before `日K` in `KlineChart` period options.
- [ ] Keep default period as `day`.
- [ ] Run `npm run typecheck` and `npm run build`.
