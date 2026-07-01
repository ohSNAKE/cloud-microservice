import { invoke } from "@tauri-apps/api/core";
import type {
  Account,
  Category,
  DashboardSummary,
  Holding,
  KlineData,
  KlinePeriod,
  MonthlyStat,
  NewHolding,
  NewTransaction,
  QuoteRefreshResult,
  Settings,
  Transaction,
} from "../types";

export const api = {
  listAccounts: () => invoke<Account[]>("list_accounts"),
  listCategories: (type?: string) => invoke<Category[]>("list_categories", { kind: type }),
  listTransactions: (month?: string) => invoke<Transaction[]>("list_transactions", { month }),
  addTransaction: (input: NewTransaction) => invoke<Transaction>("add_transaction", { input }),
  deleteTransaction: (id: number) => invoke<void>("delete_transaction", { id }),

  getDashboard: () => invoke<DashboardSummary>("get_dashboard"),
  getMonthlyStats: (months?: number) => invoke<MonthlyStat[]>("get_monthly_stats", { months }),

  listHoldings: () => invoke<Holding[]>("list_holdings"),
  addHolding: (input: NewHolding) => invoke<Holding>("add_holding", { input }),
  updateHolding: (id: number, quantity: number, costPrice: number) =>
    invoke<Holding>("update_holding", { id, quantity, costPrice }),
  deleteHolding: (id: number) => invoke<void>("delete_holding", { id }),
  refreshQuotes: () => invoke<QuoteRefreshResult>("refresh_quotes"),
  getKlineData: (code: string, kind: string, period?: KlinePeriod, limit?: number) =>
    invoke<KlineData>("get_kline_data", { code, kind, period, limit }),

  getSettings: () => invoke<Settings>("get_settings"),
  updateSettings: (settings: Settings) => invoke<Settings>("update_settings", { settings }),
  getLastSyncAt: () => invoke<string | null>("get_last_sync_at"),
};

export function formatMoney(value: number, currency = "CNY") {
  return new Intl.NumberFormat("zh-CN", {
    style: "currency",
    currency,
    minimumFractionDigits: 2,
  }).format(value);
}

export function formatPercent(value: number) {
  const prefix = value > 0 ? "+" : "";
  return `${prefix}${value.toFixed(2)}%`;
}
