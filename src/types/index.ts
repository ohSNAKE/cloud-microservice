export interface Account {
  id: number;
  name: string;
  type: string;
  balance: number;
  created_at: string;
}

export interface Category {
  id: number;
  name: string;
  type: "income" | "expense";
  icon: string;
}

export interface Transaction {
  id: number;
  type: "income" | "expense";
  amount: number;
  category_id: number | null;
  account_id: number | null;
  note: string;
  transaction_date: string;
  created_at: string;
  category_name: string | null;
  category_icon: string | null;
  account_name: string | null;
}

export interface NewTransaction {
  type: "income" | "expense";
  amount: number;
  category_id?: number;
  account_id?: number;
  note?: string;
  transaction_date: string;
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
  account_balance: number;
  holding_value: number;
  holding_profit: number;
  month_income: number;
  month_expense: number;
  month_balance: number;
  holding_count: number;
  transaction_count: number;
}

export interface MonthlyStat {
  month: string;
  income: number;
  expense: number;
}

export interface Settings {
  quote_update_interval: number;
  quote_update_enabled: boolean;
  currency: string;
}

export interface QuoteRefreshResult {
  updated: number;
  failed: number;
  message: string;
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
