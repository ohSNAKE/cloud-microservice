import { describe, expect, it } from "vitest";
import type { ScreenerDashboard, ScreenerResult } from "../types";
import { defaultScreenerSort, formatCnyBillion, formatNullableNumber, formatPercentValue, screenerState } from "./screenerView";

function row(code: string, exchange: "sh" | "sz" | "bj", daily: number | null, weekly: number | null): ScreenerResult {
  return {
    code,
    exchange,
    name: "测试",
    market_cap_cny: 50_000_000_000,
    cash_dividend_per_share: 1,
    dividend_yield: 0.05,
    current_price: 20,
    price_observed_at: "2026-07-21T01:00:00Z",
    daily_lower_band: daily === null ? null : 19.8,
    daily_distance: daily,
    daily_kline_completed_at: null,
    weekly_lower_band: weekly === null ? null : 19.6,
    weekly_distance: weekly,
    weekly_kline_completed_at: null,
    matched_periods: ["day"],
    source_metadata: "{}",
    fundamental_observed_at: "2026-07-21T01:00:00Z",
  };
}

function dashboard(overrides: Partial<ScreenerDashboard> = {}): ScreenerDashboard {
  return {
    displayed_run: null,
    latest_failed_attempt: null,
    is_stale: false,
    results: [],
    ...overrides,
  };
}

describe("screenerView", () => {
  it("orders by the smallest available lower-band distance then code and exchange", () => {
    expect(defaultScreenerSort([row("600001", "sh", 0.01, null), row("000001", "sz", 0.01, null)]).map((item) => item.code)).toEqual([
      "000001",
      "600001",
    ]);
    expect(defaultScreenerSort([row("600002", "sh", 0.05, 0.02), row("600001", "sh", 0.03, null)]).map((item) => item.code)).toEqual([
      "600002",
      "600001",
    ]);
  });

  it("distinguishes first use, no match, stale, ready, and failed states", () => {
    expect(screenerState(dashboard())).toBe("first-use");
    expect(screenerState(dashboard({ latest_failed_attempt: { started_at: "a", completed_at: "b", failure_summary: { code: "provider_unavailable", message: "timeout", skipped_count: 0 } } }))).toBe("failed");
    expect(screenerState(dashboard({ displayed_run: { id: 1, status: "success", started_at: "a", completed_at: "b", candidate_count: 1, match_count: 0, skipped_count: 0, failure_summary: null } }))).toBe("no-match");
    expect(screenerState(dashboard({ displayed_run: { id: 1, status: "success", started_at: "a", completed_at: "b", candidate_count: 1, match_count: 1, skipped_count: 0, failure_summary: null }, is_stale: true, results: [row("600001", "sh", 0.01, null)] }))).toBe("stale");
    expect(screenerState(dashboard({ displayed_run: { id: 1, status: "success", started_at: "a", completed_at: "b", candidate_count: 1, match_count: 1, skipped_count: 0, failure_summary: null }, results: [row("600001", "sh", 0.01, null)] }))).toBe("ready");
  });

  it("formats compact numeric values", () => {
    expect(formatCnyBillion(50_000_000_000)).toBe("500.00亿");
    expect(formatPercentValue(0.0523)).toBe("5.23%");
    expect(formatNullableNumber(null)).toBe("--");
  });
});
