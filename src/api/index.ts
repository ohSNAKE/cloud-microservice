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
  ParsedTransactionBatch,
  NewQuantWatchlistItem,
  QuoteRefreshResult,
  QuantDashboard,
  QuantRefreshResult,
  QuantSignal,
  QuantSignalFilter,
  QuantStrategySettingsUpdate,
  QuantTarget,
  QuantWatchlistItem,
  QuantWatchlistItemUpdate,
  ScreenerDashboard,
  ScreenerExchange,
  ScreenerRefreshSchedule,
  ScreenerWatchlistAddResult,
  Settings,
  AppLockStatus,
  Transaction,
  TransactionFilter,
  TransferInput,
  UpdateAccount,
  UpdateCategory,
  UpdateRecurringRule,
  UpdateTransaction,
} from "../types";
import { ACCOUNT_TYPES } from "../types";

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
  parseTransactionNl: (text: string) =>
    invoke<ParsedTransactionBatch>("parse_transaction_nl_command", { text }),

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
  updateRecurringRule: (id: number, input: UpdateRecurringRule) =>
    invoke<RecurringRule>("update_recurring_rule", { id, input }),
  toggleRecurringRule: (id: number, enabled: boolean) =>
    invoke<void>("toggle_recurring_rule", { id, enabled }),
  deleteRecurringRule: (id: number) => invoke<void>("delete_recurring_rule", { id }),

  listHoldings: () => invoke<Holding[]>("list_holdings"),
  addHolding: (input: NewHolding) => invoke<Holding>("add_holding", { input }),
  lookupHoldingName: (code: string, kind: string) =>
    invoke<string>("lookup_holding_name", { code, kind }),
  updateHolding: (id: number, quantity: number, costPrice: number) =>
    invoke<Holding>("update_holding", { id, quantity, costPrice }),
  deleteHolding: (id: number) => invoke<void>("delete_holding", { id }),
  refreshQuotes: () => invoke<QuoteRefreshResult>("refresh_quotes"),
  getKlineData: (code: string, kind: string, period?: KlinePeriod, limit?: number) =>
    invoke<KlineData>("get_kline_data", { code, kind, period, limit }),

  listQuantWatchlist: () => invoke<QuantWatchlistItem[]>("list_quant_watchlist"),
  addQuantWatchlist: (input: NewQuantWatchlistItem) =>
    invoke<QuantWatchlistItem>("add_quant_watchlist", { input }),
  updateQuantWatchlist: (id: number, input: QuantWatchlistItemUpdate) =>
    invoke<QuantWatchlistItem>("update_quant_watchlist", { id, input }),
  deleteQuantWatchlist: (id: number) => invoke<void>("delete_quant_watchlist", { id }),
  updateQuantStrategySettings: (
    code: string,
    market: string,
    input: QuantStrategySettingsUpdate,
  ) => invoke<QuantTarget>("update_quant_strategy_settings", { code, market, input }),
  listQuantTargets: () => invoke<QuantTarget[]>("list_quant_targets"),
  getQuantDashboard: () => invoke<QuantDashboard>("get_quant_dashboard"),
  refreshQuantSignals: () => invoke<QuantRefreshResult>("refresh_quant_signals"),
  markIntradayTSold: (code: string, market: string) =>
    invoke<void>("mark_intraday_t_sold", { code, market }),
  clearIntradayTSold: (code: string, market: string) =>
    invoke<void>("clear_intraday_t_sold", { code, market }),
  listQuantSignals: (filter?: QuantSignalFilter) =>
    invoke<QuantSignal[]>("list_quant_signals", { filter }),

  getScreenerDashboard: () => invoke<ScreenerDashboard>("get_screener_dashboard"),
  refreshScreener: () => invoke<ScreenerDashboard>("refresh_screener"),
  getScreenerRefreshSchedule: () =>
    invoke<ScreenerRefreshSchedule>("get_screener_refresh_schedule"),
  addScreenerResultToWatchlist: (code: string, exchange: ScreenerExchange, name: string) =>
    invoke<ScreenerWatchlistAddResult>("add_screener_result_to_watchlist", {
      code,
      exchange,
      name,
    }),

  getSettings: () => invoke<Settings>("get_settings"),
  updateSettings: (settings: Settings) => invoke<Settings>("update_settings", { settings }),
  listAiModels: (apiBase: string, apiKey: string) =>
    invoke<string[]>("list_ai_models_command", { apiBase, apiKey }),

  getAppLockStatus: () => invoke<AppLockStatus>("get_app_lock_status"),
  setupAppLock: (password: string) => invoke<void>("setup_app_lock", { password }),
  verifyAppLock: (password: string) => invoke<boolean>("verify_app_lock", { password }),
  changeAppLockPassword: (oldPassword: string, newPassword: string) =>
    invoke<void>("change_app_lock_password", { oldPassword, newPassword }),
  disableAppLock: (password: string) => invoke<void>("disable_app_lock", { password }),

  getLastSyncAt: () => invoke<string | null>("get_last_sync_at"),

  getDbPath: () => invoke<string>("get_db_path"),
  exportData: () => invoke<string>("export_data"),
  importData: (json: string) => invoke<void>("import_data", { json }),
};

export function formatInvokeError(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  if (error && typeof error === "object" && "message" in error) {
    return String((error as { message: unknown }).message);
  }
  return String(error);
}

export function formatMoney(value: number, currency = "CNY") {
  return new Intl.NumberFormat("zh-CN", {
    style: "currency",
    currency,
    minimumFractionDigits: 2,
  }).format(value);
}

const PRIVACY_HIDE_AMOUNTS_KEY = "caiji-privacy-hide-amounts";

export function isAmountsHidden(): boolean {
  return localStorage.getItem(PRIVACY_HIDE_AMOUNTS_KEY) === "true";
}

export function setAmountsHidden(hidden: boolean) {
  localStorage.setItem(PRIVACY_HIDE_AMOUNTS_KEY, hidden ? "true" : "false");
}

export function displayMoney(value: number, hidden: boolean, currency = "CNY") {
  return hidden ? "******" : formatMoney(value, currency);
}

export function formatPercent(value: number) {
  const prefix = value > 0 ? "+" : "";
  return `${prefix}${value.toFixed(2)}%`;
}

export function accountTypeLabel(type: string) {
  const found = ACCOUNT_TYPES.find((t) => t.value === type);
  if (found) return found.label;
  const map: Record<string, string> = {
    cash: "现金",
    bank: "银行卡",
    alipay: "支付宝",
    wechat: "微信",
    broker: "证券账户",
  };
  return map[type] ?? type;
}

export function accountTypeIconKey(type: string) {
  const found = ACCOUNT_TYPES.find((t) => t.value === type);
  return found?.icon ?? "cash";
}
