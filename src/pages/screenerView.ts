import type { ScreenerDashboard, ScreenerResult } from "../types";

export type ScreenerViewState = "first-use" | "no-match" | "ready" | "stale" | "failed";

export function smallestDistance(row: ScreenerResult): number {
  const distances = [row.daily_distance, row.weekly_distance].filter(
    (value): value is number => typeof value === "number" && Number.isFinite(value),
  );
  return distances.length > 0 ? Math.min(...distances) : Number.POSITIVE_INFINITY;
}

export function defaultScreenerSort(rows: ScreenerResult[]): ScreenerResult[] {
  return [...rows].sort((left, right) => {
    const distance = smallestDistance(left) - smallestDistance(right);
    if (distance !== 0) return distance;
    const code = left.code.localeCompare(right.code);
    if (code !== 0) return code;
    return left.exchange.localeCompare(right.exchange);
  });
}

export function screenerState(dashboard: ScreenerDashboard): ScreenerViewState {
  if (!dashboard.displayed_run) {
    return dashboard.latest_failed_attempt ? "failed" : "first-use";
  }
  if (dashboard.is_stale || dashboard.latest_failed_attempt) {
    return "stale";
  }
  return dashboard.results.length === 0 ? "no-match" : "ready";
}

export function shouldShowRefreshSuccess(dashboard: ScreenerDashboard): boolean {
  return dashboard.displayed_run?.status === "success" && !dashboard.latest_failed_attempt;
}

export function formatCnyBillion(value: number): string {
  return `${(value / 100_000_000).toFixed(2)}亿`;
}

export function formatPercentValue(value: number): string {
  return `${(value * 100).toFixed(2)}%`;
}

export function formatNullableNumber(value: number | null | undefined, digits = 2): string {
  return typeof value === "number" && Number.isFinite(value) ? value.toFixed(digits) : "--";
}
