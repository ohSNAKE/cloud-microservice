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
  it("sorts unordered valid rows, keeps the newest date, and uses the final positive close from the immediate previous date", () => {
    const result = selectIntradaySeries([
      bar("2026-07-10 09:35", 10.4),
      bar("2026-07-09 15:00", 10.1),
      bar("2026-07-10 09:30", 10.2),
      bar("2026-07-09 14:55", 10),
    ]);

    expect(result.bars.map((item) => item.date)).toEqual([
      "2026-07-10 09:30",
      "2026-07-10 09:35",
    ]);
    expect(result.previousClose).toBe(10.1);
  });

  it("discards invalid timestamps, accepts seconds, and does not fall back past an invalid immediate predecessor close", () => {
    const result = selectIntradaySeries([
      bar("bad timestamp", 9),
      bar("2026-07-08 15:00", 8.8),
      bar("2026-07-09 15:00", 0),
      bar("2026-07-09 14:55", Number.NaN),
      bar("2026-07-10 09:30:00", 10),
    ]);

    expect(result.bars.map((item) => item.date)).toEqual(["2026-07-10 09:30:00"]);
    expect(result.previousClose).toBeNull();
  });
});

describe("intraday chart helpers", () => {
  it("returns positive symmetric price and percent bounds for a flat day", () => {
    expect(getIntradayAxisBounds([bar("2026-07-10 09:30", 10)], 10)).toEqual({
      priceMin: 9.9,
      priceMax: 10.1,
      percentMin: -1,
      percentMax: 1,
    });
  });

  it("derives price and percent bounds from finite lows and highs while ignoring non-finite fields", () => {
    expect(
      getIntradayAxisBounds(
        [
          bar("2026-07-10 09:30", 10, 8, 11),
          { ...bar("2026-07-10 09:35", 10), low: Number.NaN, high: Number.POSITIVE_INFINITY },
        ],
        10,
      ),
    ).toEqual({ priceMin: 8, priceMax: 12, percentMin: -20, percentMax: 20 });
  });

  it("returns null without a previous close", () => {
    expect(getIntradayAxisBounds([bar("2026-07-10 09:30", 10)], null)).toBeNull();
  });

  it("accepts only returned session boundary labels", () => {
    expect(["09:30", "11:30", "13:00", "15:00"].every(shouldShowSessionLabel)).toBe(true);
    expect(["09:35", "11:25", "12:00", "15:05"].some(shouldShowSessionLabel)).toBe(false);
  });

  it("maps inclusive windows to returned times, separates overlapping labels, and omits empty windows", () => {
    expect(
      buildIntradayWindowAreas(
        ["09:35", "09:45", "09:50"],
        ["09:30-09:45"],
        ["09:30-09:45"],
      ),
    ).toEqual([
      {
        name: "卖T关注",
        start: "09:35",
        end: "09:45",
        labelOffset: 0,
        color: "rgba(239, 83, 80, 0.12)",
      },
      {
        name: "买回关注",
        start: "09:35",
        end: "09:45",
        labelOffset: 16,
        color: "rgba(38, 166, 154, 0.12)",
      },
    ]);
    expect(buildIntradayWindowAreas(["09:30"], ["13:00-13:30"], [])).toEqual([]);
  });
});
