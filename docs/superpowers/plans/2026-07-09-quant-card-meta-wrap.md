# Quant Card Meta Wrap Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Quant Alerts target card metadata wrap instead of showing ellipses.

**Architecture:** This is a CSS-only presentation fix. The existing React card markup remains unchanged; the metadata grid keeps two columns on desktop and one column on mobile.

**Tech Stack:** React, Ant Design, CSS, Vite, TypeScript.

---

## Chunk 1: Metadata Wrapping

### Task 1: Adjust Quant Card Metadata CSS

**Files:**
- Modify: `src/styles/global.css:1004-1022`

- [ ] **Step 1: Inspect the current truncation rule**

Confirm `.quant-target-card__meta span` uses `overflow: hidden`, `text-overflow: ellipsis`, and `white-space: nowrap`.

- [ ] **Step 2: Replace truncation with wrapping**

Set metadata spans to allow wrapping:

```css
.quant-target-card__meta span {
  min-width: 0;
  overflow-wrap: anywhere;
  white-space: normal;
}
```

- [ ] **Step 3: Verify frontend checks**

Run:

```bash
npm run typecheck
npm run build
```

Expected: both commands exit successfully. Existing Vite chunk-size warnings may remain.

- [ ] **Step 4: Manual visual verification**

Open Quant Alerts and confirm metadata fields wrap naturally instead of showing `...`.
