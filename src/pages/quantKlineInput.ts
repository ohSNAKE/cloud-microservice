import type { IntradayWindows } from "../components/intradayChartOption";
import type { Holding, QuantTarget } from "../types";

export interface KlineChartInput {
  holding: Holding;
  intradayWindows?: IntradayWindows;
}

export function targetToKlineChartInput(target: QuantTarget): KlineChartInput {
  const currentPrice = target.current_price ?? 0;
  const holding: Holding = {
    id: 0,
    code: target.code,
    name: target.name || target.code,
    type: "stock",
    quantity: 0,
    cost_price: currentPrice,
    current_price: currentPrice,
    market: target.market,
    created_at: "",
    updated_at: target.quote_fetched_at ?? "",
    market_value: 0,
    cost_value: 0,
    profit: 0,
    profit_rate: 0,
  };

  return target.strategy_mode === "intraday_t"
    ? {
        holding,
        intradayWindows: {
          sellWindows: target.intraday_high_frequency_windows,
          buybackWindows: target.intraday_low_frequency_windows,
        },
      }
    : { holding };
}
