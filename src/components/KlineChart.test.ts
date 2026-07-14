import { describe, expect, it } from "vitest";
import type { KlinePeriod } from "../types";
import * as KlineChart from "./KlineChart";

type StockKlineDisplayConfig = {
  period: KlinePeriod;
  limit: number;
  isIntraday: boolean;
};

const { getStockKlineDisplayConfig, stockKlinePeriodOptions } = KlineChart as typeof KlineChart & {
  getStockKlineDisplayConfig: (period: KlinePeriod) => StockKlineDisplayConfig;
  stockKlinePeriodOptions: Array<{ label: string; value: KlinePeriod }>;
};

describe("stock Kline display configuration", () => {
  it("exposes one-minute data for the 分时 option without exposing the quant 5m period", () => {
    expect(stockKlinePeriodOptions).toContainEqual({ label: "分时", value: "1m" });
    expect(stockKlinePeriodOptions.map((option) => option.value)).not.toContain("5m");
  });

  it("requests 500 one-minute bars for stock intraday display", () => {
    expect(getStockKlineDisplayConfig("1m")).toEqual({
      period: "1m",
      limit: 500,
      isIntraday: true,
    });
  });

  it("keeps other stock periods at 120 bars outside the intraday branch", () => {
    for (const period of ["day", "week", "month", "5m"] as const) {
      expect(getStockKlineDisplayConfig(period)).toEqual({
        period,
        limit: 120,
        isIntraday: false,
      });
    }
  });
});
