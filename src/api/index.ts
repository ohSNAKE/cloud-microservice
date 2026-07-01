import { invoke } from "@tauri-apps/api/core";
import type {
  Account,
  Budget,
  BudgetAlert,
  Category,
  CategoryStat,
  DashboardSummary,
  Holding,
  KlineData,
  KlinePeriod,
  MonthlyStat,
  NewBudget,
  NewRecurringRule,
  PortfolioHistoryPoint,
  RecurringRule,
  NewAccount,
  NewCategory,
  NewHolding,
  NewTransaction,
  QuoteRefreshResult,
  Settings,
  Transaction,
  TransactionFilter,
  TransferInput,
  UpdateAccount,
  UpdateCategory,
  UpdateTransaction,
} from "../types";

export const api = {
  listAccounts: () => invoke<Account[]>("list_accounts"),
  addAccount: (input: NewAccount) => invoke<Account>("add_account", { input }),
  updateAccount: (id: number, input: UpdateAccount) =>
    invoke<Account>("update_account", { id, input }),
  deleteAccount: (id: number) => invoke<void>("delete_account", { id }),

  listCategories: (type?: string) => invoke<Category[]>("list_categories", { kind: type }),
  addCategory: (input: NewCategory) => invoke<Category>("add_category", { input }),
  updateCategory: (id: number, input: UpdateCategory) =>
    invoke<Category>("update_category", { id, input }),
  deleteCategory: (id: number) => invoke<void>("delete_category", { id }),

  listTransactions: (filter?: TransactionFilter) =>
    invoke<Transaction[]>("list_transactions", { filter }),
  addTransaction: (input: NewTransaction) => invoke<Transaction>("add_transaction", { input }),
  updateTransaction: (id: number, input: UpdateTransaction) =>
    invoke<Transaction>("update_transaction", { id, input }),
  deleteTransaction: (id: number) => invoke<void>("delete_transaction", { id }),
  addTransfer: (input: TransferInput) => invoke<Transaction>("add_transfer", { input }),

  getDashboard: () => invoke<DashboardSummary>("get_dashboard"),
  getMonthlyStats: (months?: number) => invoke<MonthlyStat[]>("get_monthly_stats", { months }),
  getCategoryStats: (month?: string, kind?: string) =>
    invoke<CategoryStat[]>("get_category_stats", { month, kind }),
  getPortfolioHistory: (days?: number) =>
    invoke<PortfolioHistoryPoint[]>("get_portfolio_history", { days }),

  listBudgets: (month?: string) => invoke<Budget[]>("list_budgets", { month }),
  setBudget: (input: NewBudget) => invoke<Budget>("set_budget", { input }),
  deleteBudget: (id: number) => invoke<void>("delete_budget", { id }),
  getBudgetAlerts: (month?: string) => invoke<BudgetAlert[]>("get_budget_alerts", { month }),

  listRecurringRules: () => invoke<RecurringRule[]>("list_recurring_rules"),
  addRecurringRule: (input: NewRecurringRule) => invoke<RecurringRule>("add_recurring_rule", { input }),
  toggleRecurringRule: (id: number, enabled: boolean) =>
    invoke<void>("toggle_recurring_rule", { id, enabled }),
  deleteRecurringRule: (id: number) => invoke<void>("delete_recurring_rule", { id }),

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

  getDbPath: () => invoke<string>("get_db_path"),
  exportData: () => invoke<string>("export_data"),
  importData: (json: string) => invoke<void>("import_data", { json }),
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

export function accountTypeLabel(type: string) {
  const map: Record<string, string> = {
    cash: "现金",
    bank: "银行卡",
    alipay: "支付宝",
    wechat: "微信",
    broker: "证券账户",
  };
  return map[type] ?? type;
}
