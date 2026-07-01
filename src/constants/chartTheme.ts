import type { ResolvedTheme } from "../theme/types";

const CHART_COLORS_LIGHT = {
  income: "#52c41a",
  expense: "#ff4d4f",
  primary: "#1677ff",
  primaryArea: "rgba(22, 119, 255, 0.12)",
  candleUp: "#ef5350",
  candleDown: "#26a69a",
  palette: ["#1677ff", "#69b1ff", "#95de64", "#ffc53d", "#b37feb"],
  expensePalette: ["#ff4d4f", "#ff7875", "#ffa39e", "#ffccc7", "#ffd666", "#ffc53d"],
  textSecondary: "#667085",
  textMuted: "#98a2b3",
  gridLine: "#eef2f7",
};

const CHART_COLORS_DARK = {
  income: "#73d13d",
  expense: "#ff7875",
  primary: "#4096ff",
  primaryArea: "rgba(64, 150, 255, 0.18)",
  candleUp: "#ef5350",
  candleDown: "#26a69a",
  palette: ["#4096ff", "#69b1ff", "#95de64", "#ffc53d", "#b37feb"],
  expensePalette: ["#ff7875", "#ff9c6e", "#ffa39e", "#ffccc7", "#ffd666", "#ffc53d"],
  textSecondary: "#9aa0a6",
  textMuted: "#6b7280",
  gridLine: "#2a3140",
};

export type ChartColors = typeof CHART_COLORS_LIGHT | typeof CHART_COLORS_DARK;

export function getChartColors(resolved: ResolvedTheme): ChartColors {
  return resolved === "dark" ? CHART_COLORS_DARK : CHART_COLORS_LIGHT;
}

/** @deprecated 请使用 getChartColors(resolved) 或 useTheme().chartColors */
export const CHART_COLORS = CHART_COLORS_LIGHT;

export const chartGrid = { left: 48, right: 24, top: 48, bottom: 32 };
