# Quant Card K-Line Entry Design

## Goal

Add a K-line chart entry to each Quant Alerts target card.

## Approved Approach

Use option A: place a small `LineChartOutlined` icon button in the card header next to the status tag. Clicking it opens a modal on the current Quant Alerts page and reuses the existing `KlineChart` component.

## Scope

- Frontend only.
- Reuse existing K-line fetching and chart rendering through `KlineChart`.
- Do not change quant strategy rules, refresh logic, or backend APIs.

## Data Mapping

`KlineChart` accepts a `Holding`, but quant targets do not have holding quantity/cost fields. The Quant Alerts page will create a temporary chart holding from the target:

- `code`: target code
- `name`: target name or code
- `type`: `stock`
- `market`: target market
- `current_price`: target current price or `0`
- `cost_price`: target current price or `0`, used only as a chart reference line
- numeric portfolio fields: `0`

## Verification

- `npm run typecheck`
- `npm run build`
- Manual check: clicking the icon opens the K-line modal for that card.
