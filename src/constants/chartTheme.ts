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
  income: "#6bcb77",
  expense: "#f07178",
  primary: "#6aabff",
  primaryArea: "rgba(106, 171, 255, 0.16)",
  candleUp: "#f07178",
  candleDown: "#6bcb77",
  palette: ["#6aabff", "#7eb8ff", "#6bcb77", "#e6b85c", "#b896e8"],
  expensePalette: ["#f07178", "#f0898f", "#f5a5a9", "#f8c1c4", "#e6b85c", "#d4a84b"],
  textSecondary: "#9aa3af",
  textMuted: "#636b78",
  gridLine: "rgba(255, 255, 255, 0.06)",
};

export type ChartColors = typeof CHART_COLORS_LIGHT | typeof CHART_COLORS_DARK;

export function getChartColors(resolved: ResolvedTheme): ChartColors {
  return resolved === "dark" ? CHART_COLORS_DARK : CHART_COLORS_LIGHT;
}

/** @deprecated 请使用 getChartColors(resolved) 或 useTheme().chartColors */
export const CHART_COLORS = CHART_COLORS_LIGHT;

export const chartGrid = { left: 48, right: 24, top: 48, bottom: 32 };
