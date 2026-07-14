import { useEffect, useMemo, useState } from "react";
import { Radio, Spin } from "antd";
import ReactECharts from "echarts-for-react";
import { api } from "../api";
import type { ChartColors } from "../constants/chartTheme";
import { useTheme } from "../context/ThemeContext";
import type { Holding, KlineBar, KlinePeriod } from "../types";
import { buildIntradayOption, type IntradayWindows } from "./intradayChartOption";

interface KlineChartProps {
  holding: Holding;
  intradayWindows?: IntradayWindows;
}

function formatVolume(value: number) {
  if (value >= 100_000_000) return `${(value / 100_000_000).toFixed(2)}亿`;
  if (value >= 10_000) return `${(value / 10_000).toFixed(2)}万`;
  return value.toFixed(0);
}

function buildCandlestickOption(bars: KlineBar[], costPrice: number, colors: ChartColors) {
  const dates = bars.map((b) => b.date);
  const ohlc = bars.map((b) => [b.open, b.close, b.low, b.high]);
  const volumes = bars.map((b, i) => {
    const up = bars[i].close >= bars[i].open;
    return { value: b.volume, itemStyle: { color: up ? colors.candleUp : colors.candleDown } };
  });

  return {
    animation: false,
    tooltip: {
      trigger: "axis",
      axisPointer: { type: "cross" },
      formatter: (params: Array<{ axisValue: string; data: number[]; seriesName: string; value: number }>) => {
        const candle = params.find((p) => p.seriesName === "K线");
        const vol = params.find((p) => p.seriesName === "成交量");
        if (!candle) return "";
        const [open, close, low, high] = candle.data;
        return [
          `<strong>${candle.axisValue}</strong>`,
          `开：${open.toFixed(2)}`,
          `收：${close.toFixed(2)}`,
          `高：${high.toFixed(2)}`,
          `低：${low.toFixed(2)}`,
          vol ? `量：${formatVolume(vol.value)}` : "",
        ].join("<br/>");
      },
    },
    axisPointer: { link: [{ xAxisIndex: "all" }] },
    grid: [
      { left: 56, right: 20, top: 24, height: "58%" },
      { left: 56, right: 20, top: "76%", height: "14%" },
    ],
    xAxis: [
      { type: "category", data: dates, boundaryGap: true, axisLine: { onZero: false }, gridIndex: 0 },
      { type: "category", data: dates, gridIndex: 1, axisLabel: { show: false } },
    ],
    yAxis: [
      { scale: true, splitArea: { show: true }, gridIndex: 0 },
      { scale: true, gridIndex: 1, splitNumber: 2, axisLabel: { formatter: formatVolume } },
    ],
    dataZoom: [
      { type: "inside", xAxisIndex: [0, 1], start: 60, end: 100 },
      { show: true, xAxisIndex: [0, 1], type: "slider", bottom: 4, height: 18, start: 60, end: 100 },
    ],
    series: [
      {
        name: "K线",
        type: "candlestick",
        data: ohlc,
        xAxisIndex: 0,
        yAxisIndex: 0,
        itemStyle: {
          color: colors.candleUp,
          color0: colors.candleDown,
          borderColor: colors.candleUp,
          borderColor0: colors.candleDown,
        },
        markLine: {
          symbol: "none",
          lineStyle: { type: "dashed", color: "#faad14" },
          label: { formatter: "成本价" },
          data: [{ yAxis: costPrice }],
        },
      },
      {
        name: "成交量",
        type: "bar",
        data: volumes,
        xAxisIndex: 1,
        yAxisIndex: 1,
      },
    ],
  };
}

function buildLineOption(bars: KlineBar[], costPrice: number, colors: ChartColors) {
  const dates = bars.map((b) => b.date);
  const values = bars.map((b) => b.close);

  return {
    animation: false,
    tooltip: {
      trigger: "axis",
      formatter: (params: Array<{ axisValue: string; value: number }>) => {
        const point = params[0];
        if (!point) return "";
        return `<strong>${point.axisValue}</strong><br/>净值：${point.value.toFixed(4)}`;
      },
    },
    grid: { left: 56, right: 20, top: 24, bottom: 48 },
    xAxis: { type: "category", data: dates, boundaryGap: false },
    yAxis: { scale: true, splitArea: { show: true } },
    dataZoom: [
      { type: "inside", start: 60, end: 100 },
      { show: true, type: "slider", bottom: 4, height: 18, start: 60, end: 100 },
    ],
    series: [
      {
        name: "净值",
        type: "line",
        data: values,
        smooth: true,
        showSymbol: false,
        lineStyle: { width: 2, color: colors.primary },
        areaStyle: { color: colors.primaryArea },
        markLine: {
          symbol: "none",
          lineStyle: { type: "dashed", color: "#faad14" },
          label: { formatter: "成本价" },
          data: [{ yAxis: costPrice }],
        },
      },
    ],
  };
}

export default function KlineChart({ holding, intradayWindows }: KlineChartProps) {
  const { chartColors } = useTheme();
  const [loading, setLoading] = useState(true);
  const [period, setPeriod] = useState<KlinePeriod>("day");
  const [bars, setBars] = useState<KlineBar[]>([]);
  const [chartType, setChartType] = useState<"candlestick" | "line">("candlestick");

  useEffect(() => {
    let cancelled = false;
    (async () => {
      setLoading(true);
      try {
        const data = await api.getKlineData(
          holding.code,
          holding.type,
          holding.type === "stock" ? period : undefined,
          holding.type === "stock" && period === "1m" ? 500 : 120,
        );
        if (!cancelled) {
          setBars(data.bars);
          setChartType(data.chart_type);
        }
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [holding.code, holding.type, period]);

  const option = useMemo(() => {
    if (bars.length === 0) return {};
    if (holding.type === "stock" && period === "1m") {
      return buildIntradayOption(bars, chartColors, intradayWindows);
    }
    return chartType === "candlestick"
      ? buildCandlestickOption(bars, holding.cost_price, chartColors)
      : buildLineOption(bars, holding.cost_price, chartColors);
  }, [bars, chartType, holding.cost_price, holding.type, period, chartColors, intradayWindows]);

  if (loading) {
    return (
      <div style={{ textAlign: "center", padding: 80 }}>
        <Spin size="large" tip="加载K线数据..." />
      </div>
    );
  }

  if (bars.length === 0 || option === null) {
    return <div style={{ textAlign: "center", padding: 80, color: chartColors.textMuted }}>暂无行情数据</div>;
  }

  return (
    <div>
      {holding.type === "stock" && (
        <Radio.Group
          value={period}
          onChange={(e) => setPeriod(e.target.value)}
          style={{ marginBottom: 12 }}
          optionType="button"
          buttonStyle="solid"
          options={[
            { label: "分时", value: "1m" },
            { label: "日K", value: "day" },
            { label: "周K", value: "week" },
            { label: "月K", value: "month" },
          ]}
        />
      )}
      {holding.type === "fund" && (
        <div style={{ marginBottom: 12, color: chartColors.textSecondary }}>基金展示净值走势（日频）</div>
      )}
      <ReactECharts option={option} style={{ height: 460 }} notMerge />
    </div>
  );
}
