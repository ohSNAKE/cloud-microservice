# Intraday Notification Wording Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Clarify intraday T desktop notifications by distinguishing historical high-point and low-point windows.

**Architecture:** The backend remains the source of generated notification bodies. The frontend fallback body mirrors the backend direction-specific wording for robustness when older or partial payloads lack `notification_body`.

**Tech Stack:** Rust/Tauri commands, React/TypeScript notification utility, Cargo tests, Vite build.

---

## Chunk 1: Direction-Specific Notification Copy

### Task 1: Backend Notification Body

**Files:**
- Modify: `src-tauri/src/commands/quant.rs`

- [ ] Add failing assertions for sell T body containing `历史高点高发时段` and `可关注卖T`.
- [ ] Add failing test for buyback body containing `历史低点高发时段` and `可关注买回`.
- [ ] Update `refresh_quant_signals_with_snapshots` notification body construction based on `target.output_state`.
- [ ] Run targeted Cargo tests.

### Task 2: Frontend Fallback Body

**Files:**
- Modify: `src/utils/quantNotifications.ts`

- [ ] Replace generic intraday fallback with direction-specific copy.
- [ ] Run `npm run typecheck` and `npm run build`.
