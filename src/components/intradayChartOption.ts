import type { EChartsOption } from "echarts";
import type { ChartColors } from "../constants/chartTheme";
import type { KlineBar, QuantTimeBucket } from "../types";
import {
  buildIntradayWindowAreas,
  getIntradayAxisBounds,
  selectIntradaySeries,
  shouldShowSessionLabel,
} from "./intradayChart";

export interface IntradayWindows {
  sellWindows: QuantTimeBucket[];
  buybackWindows: QuantTimeBucket[];
}

function formatVolume(value: number) {
  if (value >= 100_000_000) return `${(value / 100_000_000).toFixed(2)}亿`;
  if (value >= 10_000) return `${(value / 10_000).toFixed(2)}万`;
  return value.toFixed(0);
}

export function buildIntradayOption(
  bars: KlineBar[],
  colors: ChartColors,
  intradayWindows?: IntradayWindows,
): EChartsOption | null {
  const selected = selectIntradaySeries(bars);
  if (selected.bars.length === 0) return null;

  const times = selected.bars.map((bar) => bar.date.slice(11, 16));
  const bounds = getIntradayAxisBounds(selected.bars, selected.previousClose);
  const areas = intradayWindows
    ? buildIntradayWindowAreas(times, intradayWindows.sellWindows, intradayWindows.buybackWindows)
    : [];
  const latestClose = selected.bars[selected.bars.length - 1].close;
  const lineColor =
    selected.previousClose === null
      ? colors.primary
      : latestClose >= selected.previousClose
        ? colors.candleUp
        : colors.candleDown;

  return {
    animation: false,
    tooltip: {
      trigger: "axis",
      axisPointer: { type: "cross" },
      formatter: (params: unknown) => {
        const points = (Array.isArray(params) ? params : [params]) as Array<{
          axisValue?: string;
          seriesName?: string;
          value?: unknown;
        }>;
        const price = points.find((point) => point.seriesName === "分时");
        const volume = points.find((point) => point.seriesName === "成交量");
        if (!price || typeof price.value !== "number") return "";

        const lines = [`<strong>${price.axisValue ?? ""}</strong>`, `价：${price.value.toFixed(2)}`];
        if (selected.previousClose !== null) {
          const change = ((price.value - selected.previousClose) / selected.previousClose) * 100;
          lines.push(`涨跌：${change.toFixed(2)}%`);
        }
        if (volume && typeof volume.value === "number") lines.push(`量：${formatVolume(volume.value)}`);
        return lines.join("<br/>");
      },
    },
    axisPointer: { link: [{ xAxisIndex: "all" }] },
    grid: [
      { left: 56, right: 56, top: 24, height: "60%" },
      { left: 56, right: 56, top: "75%", height: "14%" },
    ],
    dataZoom: [],
    xAxis: [
      {
        type: "category",
        data: times,
        boundaryGap: false,
        axisLine: { onZero: false },
        axisLabel: { formatter: (value: string) => (shouldShowSessionLabel(value) ? value : "") },
        gridIndex: 0,
      },
      {
        type: "category",
        data: times,
        boundaryGap: false,
        axisLabel: { show: false },
        gridIndex: 1,
      },
    ],
    yAxis: [
      {
        type: "value",
        gridIndex: 0,
        scale: true,
        splitArea: { show: true },
        ...(bounds ? { min: bounds.priceMin, max: bounds.priceMax } : {}),
      },
      {
        type: "value",
        gridIndex: 0,
        position: "right",
        show: bounds !== null,
        ...(bounds
          ? {
              min: bounds.percentMin,
              max: bounds.percentMax,
              axisLabel: { formatter: (value: number) => `${value.toFixed(2)}%` },
            }
          : {}),
      },
      {
        type: "value",
        gridIndex: 1,
        scale: true,
        splitNumber: 2,
        axisLabel: { formatter: formatVolume },
      },
    ],
    series: [
      {
        name: "分时",
        type: "line",
        data: selected.bars.map((bar) => bar.close),
        xAxisIndex: 0,
        yAxisIndex: 0,
        showSymbol: false,
        connectNulls: false,
        lineStyle: { width: 2, color: lineColor },
        ...(selected.previousClose !== null
          ? {
              markLine: {
                symbol: "none",
                lineStyle: { type: "dashed", color: colors.textSecondary },
                label: { formatter: "昨收" },
                data: [{ yAxis: selected.previousClose }],
              },
            }
          : {}),
        ...(areas.length > 0
          ? {
              markArea: {
                silent: true,
                data: areas.map((area) => [
                  {
                    name: area.name,
                    xAxis: area.start,
                    itemStyle: { color: area.color },
                    label: { offset: [0, area.labelOffset] },
                  },
                  { xAxis: area.end },
                ]),
              },
            }
          : {}),
      },
      {
        name: "成交量",
        type: "bar",
        data: selected.bars.map((bar) => ({
          value: bar.volume,
          itemStyle: { color: bar.close >= bar.open ? colors.candleUp : colors.candleDown },
        })),
        xAxisIndex: 1,
        yAxisIndex: 2,
      },
    ],
  };
}
