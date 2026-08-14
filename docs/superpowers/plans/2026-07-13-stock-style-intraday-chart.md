# Stock-Style Intraday Chart Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the stock `分时` 5-minute candlestick view with a single-day stock-software-style price line, volume panel, previous-close reference, and intraday-T attention windows.

**Architecture:** Extract all timestamp parsing, day selection, prior-close, axis-bound, and attention-window conversion into a dependency-free `intradayChart` module that can run under Vitest. `KlineChart` consumes those helpers to build the ECharts option, while Quant Alerts supplies optional window data only when opening a chart for an `intraday_t` target.

**Tech Stack:** React 19, TypeScript, ECharts 5, echarts-for-react, Ant Design, Vitest.

---

## File Structure

- `package.json`: Add the frontend unit-test command and Vitest development dependency.
- `package-lock.json`: Lock the Vitest dependency graph.
- `src/components/intradayChart.ts`: Pure intraday data selection, axis calculations, session labels, and quant-window conversion.
- `src/components/intradayChart.test.ts`: Unit coverage for every pure chart-data edge case.
- `src/components/intradayChartOption.ts`: Pure ECharts option construction for a renderable intraday data set.
- `src/components/intradayChartOption.test.ts`: Option-level coverage for axes, colors, overlays, and tooltips.
- `src/components/KlineChart.tsx`: Select the intraday ECharts option for stocks, accept optional overlay input, and retain all daily/weekly/monthly/fund behavior.
- `src/pages/quantKlineInput.ts`: Convert a quant target into holding and optional intraday-window chart input.
- `src/pages/quantKlineInput.test.ts`: Unit coverage for the quant chart-input conversion.
- `src/pages/QuantAlerts.tsx`: Preserve intraday-T window arrays together with the holding-like modal chart input.

## Chunk 1: Testable Intraday Data Model

### Task 1: Add the frontend test runner

**Files:**
- Modify: `package.json`
- Modify: `package-lock.json`

- [ ] **Step 1: Add the failing test command and Vitest dependency declaration**

Update `package.json` as follows:

```json
{
  "scripts": {
    "test": "vitest run"
  },
  "devDependencies": {
    "vitest": "^3.2.4"
  }
}
```

- [ ] **Step 2: Install the declared dependency**

Run: `npm install`

Expected: `package-lock.json` records `vitest` and `npm run test` can resolve the executable.

- [ ] **Step 3: Verify the runner starts before any tests exist**

Run: `npm run test`

Expected: exit code `1` with Vitest reporting that no test files were found.

- [ ] **Step 4: Commit the test tooling setup**

```bash
git add package.json package-lock.json
git commit -m "test: add frontend test runner"
```

Expected: one commit containing only the Vitest script and lockfile changes.

### Task 2: Define pure intraday data transformations with failing tests

**Files:**
- Create: `src/components/intradayChart.test.ts`
- Create: `src/components/intradayChart.ts`

- [ ] **Step 1: Write the failing tests for latest-day selection and prior close**

Create `src/components/intradayChart.test.ts` with fixture bars using provider timestamps. Assert that `selectIntradaySeries` sorts unordered bars, keeps only the newest date, and takes the last positive close from the immediate preceding date:

```ts
import { describe, expect, it } from "vitest";
import type { KlineBar } from "../types";
import {
  buildIntradayWindowAreas,
  getIntradayAxisBounds,
  selectIntradaySeries,
  shouldShowSessionLabel,
} from "./intradayChart";

const bar = (date: string, close: number, low = close, high = close): KlineBar => ({
  date,
  open: close,
  close,
  low,
  high,
  volume: 100,
  change_pct: 0,
});

describe("selectIntradaySeries", () => {
  it("keeps the newest date in timestamp order and derives the preceding close", () => {
    const result = selectIntradaySeries([
      bar("2026-07-10 09:35", 10.4),
      bar("2026-07-09 15:00", 10.1),
      bar("2026-07-10 09:30", 10.2),
      bar("2026-07-09 14:55", 10.0),
    ]);

    expect(result.bars.map((item) => item.date)).toEqual([
      "2026-07-10 09:30",
      "2026-07-10 09:35",
    ]);
    expect(result.previousClose).toBe(10.1);
  });

  it("drops malformed timestamps and omits a zero or missing previous close", () => {
    const result = selectIntradaySeries([
      bar("bad timestamp", 9),
      bar("2026-07-08 15:00", 8.8),
      bar("2026-07-09 15:00", 0),
      bar("2026-07-10 09:30:00", 10),
    ]);

    expect(result.bars).toHaveLength(1);
    expect(result.previousClose).toBeNull();
  });
});
```

- [ ] **Step 2: Add failing tests for the symmetric axes, labels, and overlays**

Append tests that lock down the remaining specification boundaries:

```ts
describe("intraday chart helpers", () => {
  it("creates a positive symmetric price and percentage range for a flat day", () => {
    expect(getIntradayAxisBounds([bar("2026-07-10 09:30", 10)], 10)).toEqual({
      priceMin: 9.9,
      priceMax: 10.1,
      percentMin: -1,
      percentMax: 1,
    });
  });

  it("uses finite latest-day highs and lows to derive both synchronized ranges", () => {
    const result = getIntradayAxisBounds(
      [
        bar("2026-07-10 09:30", 10, 8, 11),
        { ...bar("2026-07-10 09:35", 10), low: Number.NaN, high: Number.POSITIVE_INFINITY },
      ],
      10,
    );

    expect(result).toEqual({ priceMin: 8, priceMax: 12, percentMin: -20, percentMax: 20 });
  });

  it("omits synchronized axes when no prior close is available", () => {
    expect(getIntradayAxisBounds([bar("2026-07-10 09:30", 10)], null)).toBeNull();
  });

  it("labels only the returned session boundary bars", () => {
    expect(shouldShowSessionLabel("09:30")).toBe(true);
    expect(shouldShowSessionLabel("11:25")).toBe(false);
    expect(shouldShowSessionLabel("15:00")).toBe(true);
  });

  it("maps inclusive sell and buyback windows and separates overlapping labels", () => {
    const areas = buildIntradayWindowAreas(
      ["09:30", "09:35", "09:45", "09:50"],
      ["09:30-09:45"],
      ["09:30-09:45"],
    );

    expect(areas).toMatchObject([
      { name: "卖T关注", start: "09:30", end: "09:45", labelOffset: 0, color: "rgba(239, 83, 80, 0.12)" },
      { name: "买回关注", start: "09:30", end: "09:45", labelOffset: 16, color: "rgba(38, 166, 154, 0.12)" },
    ]);
  });

  it("skips a window that has no returned bars", () => {
    expect(buildIntradayWindowAreas(["09:30"], ["13:00-13:30"], [])).toEqual([]);
  });
});
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `npm run test -- src/components/intradayChart.test.ts`

Expected: FAIL because `./intradayChart` does not exist.

- [ ] **Step 4: Implement the minimal pure module**

Create `src/components/intradayChart.ts`. Export these narrow, UI-independent contracts:

```ts
export interface IntradaySeries {
  bars: KlineBar[];
  previousClose: number | null;
}

export interface IntradayAxisBounds {
  priceMin: number;
  priceMax: number;
  percentMin: number;
  percentMax: number;
}

export interface IntradayWindowArea {
  name: "卖T关注" | "买回关注";
  start: string;
  end: string;
  labelOffset: number;
  color: string;
}
```

Declare `getIntradayAxisBounds(bars, previousClose): IntradayAxisBounds | null`. Implement `selectIntradaySeries` with a `YYYY-MM-DD HH:mm` / `YYYY-MM-DD HH:mm:ss` regular expression. Treat captured date/time components as Shanghai wall-clock strings, sort lexically by the original valid timestamp, select the final date, and find the final finite `close > 0` from the prior distinct date. Return `previousClose: null` when either date is unavailable. Do not fall back across an immediate predecessor date whose bars have no valid close.

Implement `getIntradayAxisBounds` to return `null` when `previousClose` is `null`; otherwise compute `d = max(max(abs(low - previousClose), abs(high - previousClose)), previousClose * 0.01)` across displayed bars and return the four exact bounds specified in the design. Only include finite `low` and `high` values in the maximum.

Implement `shouldShowSessionLabel(time)` as membership in `09:30`, `11:30`, `13:00`, and `15:00`. Implement `buildIntradayWindowAreas` by splitting a window at `-`, filtering the returned time list inclusively between its start and end, and producing no entry when the filtered list is empty. Emit sell areas first with `rgba(239, 83, 80, 0.12)` and offset `0`; emit buyback areas second with `rgba(38, 166, 154, 0.12)` and offset `16` whenever sell and buyback areas overlap, otherwise offset `0`.

- [ ] **Step 5: Run the focused tests to verify they pass**

Run: `npm run test -- src/components/intradayChart.test.ts`

Expected: PASS with seven passing tests.

- [ ] **Step 6: Commit the tested chart data module**

```bash
git add src/components/intradayChart.ts src/components/intradayChart.test.ts
git commit -m "feat: add intraday chart data helpers"
```

Expected: one commit containing the pure module and its passing tests.

## Chunk 2: Render And Integrate The Intraday View

### Task 3: Build the stock-style intraday ECharts option

**Files:**
- Create: `src/components/intradayChartOption.ts`
- Create: `src/components/intradayChartOption.test.ts`
- Modify: `src/components/KlineChart.tsx:1-209`

- [ ] **Step 1: Add a failing option-shape test for the chart-specific adapter**

Create `src/components/intradayChartOption.test.ts` and import the not-yet-created `buildIntradayOption` from `./intradayChartOption` plus `getChartColors` from `../constants/chartTheme`; call `getChartColors("light")` for the complete `ChartColors` fixture. Use a two-date fixture with a previous close of `10`, one rising volume bar, one falling volume bar, and an intraday window. After building the option, use these exact strict-TypeScript guards before any indexed access:

```ts
if (option === null) throw new Error("expected a renderable intraday option");
if (!Array.isArray(option.series) || !Array.isArray(option.yAxis)) {
  throw new Error("expected series and yAxis arrays");
}
const series = option.series as Array<Record<string, unknown>>;
const yAxis = option.yAxis as Array<Record<string, unknown>>;
```

Use `series` and `yAxis`, never `option.series[index]` or `option.yAxis[index]`, in all following assertions. Assert the returned option has:

```ts
expect(series).toHaveLength(2);
expect(series[0]).toMatchObject({ type: "line", showSymbol: false });
expect(series[1]).toMatchObject({ type: "bar" });
expect(option.dataZoom).toEqual([]);
expect(yAxis).toHaveLength(3);
expect(yAxis[0]).toMatchObject({ min: 9.9, max: 10.1 });
expect(yAxis[1]).toMatchObject({ min: -1, max: 1, show: true });
expect(series[1]).toMatchObject({
  data: [
    { itemStyle: { color: colors.candleUp } },
    { itemStyle: { color: colors.candleDown } },
  ],
});
```

Also assert the first series contains a `markLine.data` entry with `yAxis: 10` and label `昨收`, that the right-axis formatter returns `0.00%` for `0`, and that an intraday-T window produces `markArea.data` with `卖T关注`. Assert an omitted `intradayWindows` argument produces no `markArea`, and no previous close produces no previous-close `markLine` with a hidden right axis. Invoke the tooltip formatter with a price and volume parameter fixture and assert it includes the selected time, `价：`, `涨跌：`, and `量：`. Finally, assert `buildIntradayOption` returns `null` when all source timestamps are invalid.

- [ ] **Step 2: Run the focused test to verify it fails**

Run: `npm run test -- src/components/intradayChartOption.test.ts`

Expected: FAIL because `./intradayChartOption` does not exist.

- [ ] **Step 3: Implement the option builder and extend `KlineChart` with an optional overlay prop**

Create `src/components/intradayChartOption.ts`, importing `EChartsOption` as a type from `echarts` and exporting `buildIntradayOption(bars: KlineBar[], colors: ChartColors, intradayWindows?: IntradayWindows): EChartsOption | null`. Import `KlineBar` and `QuantTimeBucket` explicitly from `../types`, and `ChartColors` from the existing chart-theme module. Move the `IntradayWindows` interface into this module:

Add and export the prop shape below, leaving existing callers valid:

```ts
export interface IntradayWindows {
  sellWindows: QuantTimeBucket[];
  buybackWindows: QuantTimeBucket[];
}

```

Import the helper functions and types from `intradayChart.ts`, and import `ChartColors` from the existing chart-theme module. In `KlineChart.tsx`, import `buildIntradayOption` and `IntradayWindows` from the new module, add `intradayWindows?: IntradayWindows` to `KlineChartProps`, and branch first for stock `period === "5m"`. Keep `buildCandlestickOption` for the other stock periods and `buildLineOption` for funds.

- [ ] **Step 4: Build the exact intraday option and preserve the empty state**

Use `selectIntradaySeries` and return `null` when no valid latest-day bars remain. In `KlineChart`, calculate the same selected series before the existing empty-state guard and render `暂无行情数据` when raw 5-minute bars exist but the selected series is empty. Do not pass an empty option to ECharts. Type the memo result as `EChartsOption | null | {}`; immediately before rendering, use `if (bars.length === 0 || option === null)` for the empty state. This branch narrows `option` to a non-null ECharts option for the subsequent `ReactECharts` call. Configure renderable options with:

```ts
grid: [
  { left: 56, right: 56, top: 24, height: "60%" },
  { left: 56, right: 56, top: "75%", height: "14%" },
],
axisPointer: { link: [{ xAxisIndex: "all" }] },
dataZoom: [],
```

Use category x-axes containing the selected `HH:mm` values, with an `axisLabel.formatter` that returns the value only when `shouldShowSessionLabel(value)` is true. Use three y-axes: price on grid `0`, percentage on the right of grid `0`, and volume on grid `1`. When bounds exist, set the price and percentage axes to the exact `min`/`max` values from `getIntradayAxisBounds`; format the right axis as `${value.toFixed(2)}%`. When bounds are absent, hide the percentage axis and omit the previous-close `markLine`.

Create a `line` price series from selected `close` values with `showSymbol: false`, `connectNulls: false`, and color determined from the latest price versus prior close when present (otherwise the theme primary color). Add the dashed `昨收` mark line only when a prior close exists. Convert `buildIntradayWindowAreas` output into ECharts `markArea.data` pairs with each area's color and label offset. Create a second `bar` series on grid `1`; use `close >= open ? colors.candleUp : colors.candleDown` for each volume bar. Preserve crosshair tooltips and show time, price, `涨跌：${change.toFixed(2)}%` when a prior close exists, and volume.

- [ ] **Step 5: Run focused tests and frontend type checking**

Run: `npm run test -- src/components/intradayChart.test.ts src/components/intradayChartOption.test.ts && npm run typecheck`

Expected: all intraday tests PASS and TypeScript exits `0`.

- [ ] **Step 6: Commit the chart rendering change**

```bash
git add src/components/KlineChart.tsx src/components/intradayChartOption.ts src/components/intradayChartOption.test.ts
git commit -m "feat: render stock-style intraday chart"
```

Expected: one commit containing the intraday ECharts option, its option-level tests, and the KlineChart integration.

### Task 4: Pass intraday-T windows from Quant Alerts

**Files:**
- Create: `src/pages/quantKlineInput.ts`
- Create: `src/pages/quantKlineInput.test.ts`
- Modify: `src/pages/QuantAlerts.tsx:31-45,114-132,147-160,485-491,742-750`

- [ ] **Step 1: Add a failing test for the Quant Alerts chart-input adapter**

Create `src/pages/quantKlineInput.test.ts` and import the not-yet-created `targetToKlineChartInput` from `./quantKlineInput`. Define a complete `QuantTarget` factory with these stable values: `code: "000001"`, `name: "平安银行"`, `market: "cn"`, `source: "watchlist"`, `enabled: true`, `desktop_notification_enabled: false`, `current_price: 10`, `quote_fetched_at: null`, `trend_state: "neutral"`, `output_state: "watch"`, `current_trigger_zone: null`, `ma_short: null`, `ma_long: null`, `grid_zones: []`, `intraday_position: null`, `intraday_reason: null`, `has_sold_t_today: false`, `latest_signal: null`, and `last_error: null`. Supply `strategy_mode`, `intraday_high_frequency_windows`, and `intraday_low_frequency_windows` per test.

Assert an `intraday_t` target preserves both provided window arrays, and an `auto_grid` target has `intradayWindows: undefined` while still returning the holding-like stock data.

- [ ] **Step 2: Run the focused test to verify it fails**

Run: `npm run test -- src/pages/quantKlineInput.test.ts`

Expected: FAIL because `./quantKlineInput` does not exist.

- [ ] **Step 3: Replace the holding-only Quant Alerts modal state**

Create `src/pages/quantKlineInput.ts`. Import `Holding` and `QuantTarget` from `../types` and `IntradayWindows` from `../components/intradayChartOption`. Define and export:

```ts
export interface KlineChartInput {
  holding: Holding;
  intradayWindows?: IntradayWindows;
}
```

Implement and export `targetToKlineChartInput(target: QuantTarget): KlineChartInput` by moving the existing holding conversion into this module and adding window arrays only for `target.strategy_mode === "intraday_t"`. In `QuantAlerts.tsx`, import that helper and its input type, remove the now-unused `Holding` type import, replace `klineHolding` with `klineChartInput: KlineChartInput | null`, and update the chart icon handler, modal title, open condition, close handler, and render call so the final call is:

```tsx
<KlineChart
  holding={klineChartInput.holding}
  intradayWindows={klineChartInput.intradayWindows}
/>
```

Do not change `Holdings.tsx`; its existing holding-only use intentionally produces no quantitative overlay.

- [ ] **Step 4: Run the complete verification suite**

Run: `npm run test && npm run typecheck && npm run build && cargo test --manifest-path src-tauri/Cargo.toml live_fetch_stock_five_minute_kline_002796 -- --ignored`

Expected: Vitest, TypeScript, Vite build, and the existing ignored live 5-minute backend test all exit `0`. If the live provider is unavailable, record that external-service failure separately and do not treat it as a frontend regression.

- [ ] **Step 5: Manually verify both entry points in the desktop app**

Run: `npm run tauri dev`

In the opened app, add or use A-share holding `000001`, open its K-line modal, choose `分时`, move the pointer over the line, and confirm the crosshair tooltip reports time, price, change, and volume with no colored attention bands. Open 量化提醒, add `000001` to the watchlist if needed, select 做T mode, trigger a manual refresh during a session with returned intraday windows, open its K-line modal, choose `分时`, and confirm returned sell/buyback windows are translucent bands with separate labels when both overlap. Hover a bar and confirm the same tooltip fields. Then switch through 日K, 周K, 月K, and a fund holding to confirm those existing views remain unchanged.

- [ ] **Step 6: Commit the Quant Alerts integration**

```bash
git add src/pages/QuantAlerts.tsx src/pages/quantKlineInput.ts src/pages/quantKlineInput.test.ts
git commit -m "feat: show intraday T windows on charts"
```

Expected: one commit containing the Quant Alerts chart-input integration and adapter test.
