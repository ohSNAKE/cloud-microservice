# Quant Card Meta Wrap Design

## Goal

Make the Quant Alerts target card metadata readable without ellipses, while preserving the current card structure and visual density.

## Approved Approach

Use option A from the visual review: keep the existing two-column metadata grid, but allow metadata values to wrap naturally instead of truncating with `...`.

## Scope

- Update only the Quant Alerts target card metadata styles.
- Preserve current card markup, strategy controls, signal history, and all backend signal logic.
- Keep the current responsive behavior where small screens collapse metadata to one column.

## Design Details

- Remove single-line truncation from `.quant-target-card__meta span`.
- Allow long values such as `卖T窗口`, `买回窗口`, `理由`, `触发`, and `行情` to wrap across lines.
- Keep the two-column layout on desktop so cards remain compact and familiar.
- Let cards grow vertically when content is long.

## Verification

- TypeScript typecheck should pass.
- Production build should pass.
- Manually verify the Quant Alerts page no longer shows ellipses in card metadata.
