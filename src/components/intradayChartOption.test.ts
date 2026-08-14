import { describe, expect, it } from "vitest";
import { getChartColors } from "../constants/chartTheme";
import type { KlineBar } from "../types";
import { buildIntradayOption } from "./intradayChartOption";

const colors = getChartColors("light");

const bar = (
  date: string,
  open: number,
  close: number,
  low = Math.min(open, close),
  high = Math.max(open, close),
  volume = 100,
): KlineBar => ({ date, open, close, low, high, volume, change_pct: 0 });

const bars = [
  bar("2026-07-09 15:00", 9.9, 10),
  bar("2026-07-10 09:30", 10, 10.05, 9.95, 10.05, 120),
  bar("2026-07-10 09:35", 10.05, 9.98, 9.96, 10.04, 80),
];

describe("buildIntradayOption", () => {
  it("builds the stock-style intraday option with synchronized axes, volume, and attention areas", () => {
    const option = buildIntradayOption(bars, colors, {
      sellWindows: ["09:30-09:45"],
      buybackWindows: [],
    });

    if (option === null) throw new Error("expected a renderable intraday option");
    if (!Array.isArray(option.series) || !Array.isArray(option.yAxis)) {
      throw new Error("expected series and yAxis arrays");
    }
    const series = option.series as Array<Record<string, unknown>>;
    const yAxis = option.yAxis as Array<Record<string, unknown>>;

    expect(series).toHaveLength(2);
    expect(series[0]).toMatchObject({ type: "line", showSymbol: false });
    expect(series[1]).toMatchObject({ type: "bar" });
    expect(option.dataZoom).toEqual([]);
    expect(yAxis).toHaveLength(3);
    expect(yAxis[0]).toMatchObject({ min: 9.9, max: 10.1 });
    expect(yAxis[1]).toMatchObject({ min: -1, max: 1, show: true });
    expect((yAxis[1].axisLabel as { formatter: (value: number) => string }).formatter(0)).toBe("0.00%");
    expect(series[0].markLine).toMatchObject({
      data: [{ yAxis: 10 }],
      label: { formatter: "昨收" },
    });
    expect(series[1]).toMatchObject({
      data: [
        { itemStyle: { color: colors.candleUp } },
        { itemStyle: { color: colors.candleDown } },
      ],
    });
    const markArea = series[0].markArea as { data: Array<Array<Record<string, unknown>>> };
    expect(markArea.data[0][0]).toMatchObject({ name: "卖T关注" });

    const formatter = (option.tooltip as unknown as {
      formatter: (params: Array<{ axisValue: string; seriesName: string; value: number }>) => string;
    }).formatter;
    const tooltip = formatter([
      { axisValue: "09:30", seriesName: "分时", value: 10.05 },
      { axisValue: "09:30", seriesName: "成交量", value: 120 },
    ]);
    expect(tooltip).toContain("09:30");
    expect(tooltip).toContain("价：");
    expect(tooltip).toContain("涨跌：");
    expect(tooltip).toContain("量：");
  });

  it("omits attention areas when no windows are supplied", () => {
    const option = buildIntradayOption(bars, colors);

    if (option === null || !Array.isArray(option.series)) {
      throw new Error("expected a renderable series");
    }
    const series = option.series as Array<Record<string, unknown>>;
    expect(series[0].markArea).toBeUndefined();
  });

  it("hides the percentage axis and previous-close line without a prior close", () => {
    const option = buildIntradayOption([bar("2026-07-10 09:30", 10, 10.05)], colors);

    if (option === null || !Array.isArray(option.series) || !Array.isArray(option.yAxis)) {
      throw new Error("expected a renderable option");
    }
    const series = option.series as Array<Record<string, unknown>>;
    const yAxis = option.yAxis as Array<Record<string, unknown>>;
    expect(yAxis[1]).toMatchObject({ show: false });
    expect(series[0].markLine).toBeUndefined();
  });

  it("returns null when no source timestamps are valid", () => {
    expect(buildIntradayOption([bar("invalid", 10, 10)], colors)).toBeNull();
  });
});
