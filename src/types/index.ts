export interface Account {
  id: number;
  name: string;
  type: string;
  balance: number;
  created_at: string;
}

export interface NewAccount {
  name: string;
  type: string;
  balance?: number;
}

export interface UpdateAccount {
  name: string;
  type: string;
  balance: number;
}

export interface Category {
  id: number;
  name: string;
  type: "income" | "expense";
  icon: string;
}

export interface NewCategory {
  name: string;
  type: "income" | "expense";
  icon?: string;
}

export interface UpdateCategory {
  name: string;
  icon: string;
}

export interface Transaction {
  id: number;
  type: "income" | "expense" | "transfer";
  amount: number;
  category_id: number | null;
  account_id: number | null;
  transfer_to_account_id: number | null;
  note: string;
  transaction_date: string;
  created_at: string;
  category_name: string | null;
  category_icon: string | null;
  account_name: string | null;
  transfer_to_account_name: string | null;
}

export interface NewTransaction {
  type: "income" | "expense";
  amount: number;
  category_id?: number;
  account_id?: number;
  note?: string;
  transaction_date: string;
}

export interface UpdateTransaction {
  type: "income" | "expense";
  amount: number;
  category_id?: number;
  account_id?: number;
  note?: string;
  transaction_date: string;
}

export interface TransferInput {
  from_account_id: number;
  to_account_id: number;
  amount: number;
  note?: string;
  transaction_date: string;
}

export interface TransactionFilter {
  month?: string;
  account_id?: number;
  category_id?: number;
  tx_type?: string;
  keyword?: string;
}

export interface Holding {
  id: number;
  code: string;
  name: string;
  type: "stock" | "fund";
  quantity: number;
  cost_price: number;
  current_price: number;
  market: string;
  created_at: string;
  updated_at: string;
  market_value: number;
  cost_value: number;
  profit: number;
  profit_rate: number;
}

export interface NewHolding {
  code: string;
  name: string;
  type: "stock" | "fund";
  quantity: number;
  cost_price: number;
  market?: string;
}

export interface DashboardSummary {
  total_assets: number;
  liquid_assets: number;
  broker_balance: number;
  account_balance: number;
  holding_value: number;
  holding_profit: number;
  month_income: number;
  month_expense: number;
  month_balance: number;
  holding_count: number;
  transaction_count: number;
  is_empty: boolean;
}

export interface MonthlyStat {
  month: string;
  income: number;
  expense: number;
}

export interface CategoryStat {
  category_id: number;
  category_name: string;
  category_icon: string;
  amount: number;
  percentage: number;
}

export type ThemeMode = "light" | "dark" | "system";

export interface AppLockStatus {
  enabled: boolean;
  configured: boolean;
}

export interface Settings {
  quote_update_interval: number;
  quote_update_enabled: boolean;
  refresh_on_startup: boolean;
  currency: string;
  ai_enabled: boolean;
  ai_api_key: string;
  ai_api_base: string;
  ai_model: string;
  theme_mode: ThemeMode;
}

export interface ParsedTransactionDraft {
  type: "income" | "expense";
  amount: number | null;
  category_id: number | null;
  category_name: string | null;
  account_id: number | null;
  account_name: string | null;
  transaction_date: string;
  note: string;
  confidence: number;
  source: "ai" | "rule";
  raw_text: string;
  parse_notice: string | null;
}

export interface ParsedTransactionBatch {
  raw_text: string;
  items: ParsedTransactionDraft[];
  parse_notice: string | null;
  source: "ai" | "rule";
}

export interface QuoteRefreshResult {
  updated: number;
  failed: number;
  message: string;
  failed_codes: string[];
}

export interface KlineBar {
  date: string;
  open: number;
  close: number;
  low: number;
  high: number;
  volume: number;
  change_pct: number;
}

export interface KlineData {
  bars: KlineBar[];
  chart_type: "candlestick" | "line";
}

export type KlinePeriod = "day" | "week" | "month";

export interface Budget {
  id: number;
  category_id: number;
  category_name: string;
  category_icon: string;
  month: string;
  amount: number;
  spent: number;
  remaining: number;
  usage_rate: number;
  is_over: boolean;
}

export interface NewBudget {
  category_id: number;
  month: string;
  amount: number;
}

export interface BudgetAlert {
  category_name: string;
  category_icon: string;
  budget: number;
  spent: number;
  over_amount: number;
}

export interface RecurringRule {
  id: number;
  type: "income" | "expense";
  amount: number;
  category_id: number | null;
  account_id: number | null;
  note: string;
  day_of_month: number;
  enabled: boolean;
  last_run_month: string | null;
  category_name: string | null;
  category_icon: string | null;
  account_name: string | null;
}

export interface NewRecurringRule {
  type: "income" | "expense";
  amount: number;
  category_id?: number;
  account_id?: number;
  note?: string;
  day_of_month: number;
}

export interface UpdateRecurringRule {
  type: "income" | "expense";
  amount: number;
  category_id?: number;
  account_id?: number;
  note?: string;
  day_of_month: number;
}

export interface PortfolioHistoryPoint {
  date: string;
  total_value: number;
}

export const ACCOUNT_TYPES = [
  { label: "现金", value: "cash", icon: "cash" },
  { label: "银行卡", value: "bank", icon: "bank" },
  { label: "支付宝", value: "alipay", icon: "alipay" },
  { label: "微信", value: "wechat", icon: "wechat" },
  { label: "证券账户", value: "broker", icon: "broker" },
] as const;
