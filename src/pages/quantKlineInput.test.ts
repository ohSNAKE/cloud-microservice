import { describe, expect, it } from "vitest";
import type { QuantTarget } from "../types";
import { targetToKlineChartInput } from "./quantKlineInput";

function createTarget(
  strategy_mode: QuantTarget["strategy_mode"],
  intraday_high_frequency_windows: QuantTarget["intraday_high_frequency_windows"],
  intraday_low_frequency_windows: QuantTarget["intraday_low_frequency_windows"],
): QuantTarget {
  return {
    code: "000001",
    name: "平安银行",
    market: "cn",
    source: "watchlist",
    strategy_mode,
    enabled: true,
    desktop_notification_enabled: false,
    current_price: 10,
    quote_fetched_at: null,
    trend_state: "neutral",
    output_state: "watch",
    current_trigger_zone: null,
    ma_short: null,
    ma_long: null,
    grid_zones: [],
    intraday_position: null,
    intraday_reason: null,
    intraday_high_frequency_windows,
    intraday_low_frequency_windows,
    has_sold_t_today: false,
    latest_signal: null,
    last_error: null,
  };
}

describe("targetToKlineChartInput", () => {
  it("preserves intraday-T windows and creates a stock holding input", () => {
    const sellWindows = ["09:30-09:45", "13:35-14:30"] as const;
    const buybackWindows = ["10:35-11:30"] as const;

    const input = targetToKlineChartInput(
      createTarget("intraday_t", [...sellWindows], [...buybackWindows]),
    );

    expect(input.holding).toEqual({
      id: 0,
      code: "000001",
      name: "平安银行",
      type: "stock",
      quantity: 0,
      cost_price: 10,
      current_price: 10,
      market: "cn",
      created_at: "",
      updated_at: "",
      market_value: 0,
      cost_value: 0,
      profit: 0,
      profit_rate: 0,
    });
    expect(input.intradayWindows).toEqual({ sellWindows, buybackWindows });
  });

  it("omits intraday windows for auto-grid while creating a stock holding input", () => {
    const input = targetToKlineChartInput(createTarget("auto_grid", [], []));

    expect(input.intradayWindows).toBeUndefined();
    expect(input.holding).toMatchObject({
      code: "000001",
      name: "平安银行",
      type: "stock",
      current_price: 10,
      cost_price: 10,
      quantity: 0,
    });
  });
});
