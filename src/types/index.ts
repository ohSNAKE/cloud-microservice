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

export type KlinePeriod = "1m" | "5m" | "day" | "week" | "month";

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

export type QuantStrategyMode = "auto_grid" | "intraday_t";
export type QuantGridDirection = "buy_attention" | "sell_attention";
export type QuantDirection =
  | QuantGridDirection
  | "sell_t_attention"
  | "buyback_attention";
export type QuantTrendState = "bullish" | "bearish" | "neutral" | "insufficient_data";
export type QuantOutputState = QuantDirection | "sell_t_watch" | "watch" | "quote_error";
export type QuantGridTriggerZone = "buy_1" | "buy_2" | "sell_1" | "sell_2";
export type QuantTimeBucket =
  | "09:30-09:45"
  | "09:50-10:30"
  | "10:35-11:30"
  | "13:00-13:30"
  | "13:35-14:30"
  | "14:35-15:00";
export type QuantTriggerZone = QuantGridTriggerZone | QuantTimeBucket;

export interface QuantWatchlistItem {
  id: number;
  code: string;
  name: string;
  market: string;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface NewQuantWatchlistItem {
  code: string;
  name?: string;
  market?: string;
  enabled?: boolean;
}

export interface QuantWatchlistItemUpdate {
  name: string;
  enabled: boolean;
}

export interface QuantStrategySettingsUpdate {
  enabled: boolean;
  desktop_notification_enabled: boolean;
  strategy_mode?: QuantStrategyMode;
}

export interface QuantGridZone {
  zone: QuantGridTriggerZone;
  direction: QuantGridDirection;
  lower: number;
  upper: number;
}

export interface QuantSignal {
  id: number;
  code: string;
  name: string;
  market: string;
  direction: QuantDirection;
  trigger_price: number;
  trend_state: "bullish" | "bearish" | "neutral";
  source: QuantStrategyMode;
  trigger_zone: QuantTriggerZone;
  triggered_at: string;
}

export interface QuantGeneratedSignal {
  signal: QuantSignal;
  desktop_notification_enabled: boolean;
  notification_body: string | null;
}

export interface QuantTarget {
  code: string;
  name: string;
  market: string;
  source: "holding" | "watchlist" | "holding_watchlist";
  strategy_mode: QuantStrategyMode;
  enabled: boolean;
  desktop_notification_enabled: boolean;
  current_price: number | null;
  quote_fetched_at: string | null;
  trend_state: QuantTrendState;
  output_state: QuantOutputState;
  current_trigger_zone: QuantTriggerZone | null;
  ma_short: number | null;
  ma_long: number | null;
  grid_zones: QuantGridZone[];
  intraday_position: number | null;
  intraday_reason: string | null;
  intraday_high_frequency_windows: QuantTimeBucket[];
  intraday_low_frequency_windows: QuantTimeBucket[];
  has_sold_t_today: boolean;
  latest_signal: QuantSignal | null;
  last_error: string | null;
}

export interface QuantDashboard {
  is_trading_time: boolean;
  next_refresh_at: string | null;
  targets: QuantTarget[];
  recent_signals: QuantSignal[];
}

export interface QuantRefreshResult {
  dashboard: QuantDashboard;
  generated_signals: QuantGeneratedSignal[];
}

export interface QuantSignalFilter {
  code?: string;
  direction?: QuantDirection;
  from?: string;
  to?: string;
  limit?: number;
}

export type ScreenerExchange = "sh" | "sz" | "bj";
export type ScreenerPeriod = "day" | "week";
export type ScreenerRunStatus = "success" | "partial" | "failed";
export type ScreenerFailureCode =
  | "universe_unavailable"
  | "provider_unavailable"
  | "partial_data"
  | "import_invalidated";

export interface ScreenerFailureSummary {
  code: ScreenerFailureCode | string;
  message: string;
  skipped_count: number;
}

export interface ScreenerRun {
  id: number;
  status: ScreenerRunStatus | string;
  started_at: string;
  completed_at: string;
  candidate_count: number;
  match_count: number;
  skipped_count: number;
  failure_summary: ScreenerFailureSummary | null;
}

export interface ScreenerFailedAttempt {
  started_at: string;
  completed_at: string;
  failure_summary: ScreenerFailureSummary | null;
}

export interface ScreenerResult {
  code: string;
  exchange: ScreenerExchange;
  name: string;
  market_cap_cny: number;
  cash_dividend_per_share: number;
  dividend_yield: number;
  current_price: number;
  price_observed_at: string;
  daily_lower_band: number | null;
  daily_distance: number | null;
  daily_kline_completed_at: string | null;
  weekly_lower_band: number | null;
  weekly_distance: number | null;
  weekly_kline_completed_at: string | null;
  matched_periods: ScreenerPeriod[];
  source_metadata: string;
  fundamental_observed_at: string;
}

export interface ScreenerDashboard {
  displayed_run: ScreenerRun | null;
  latest_failed_attempt: ScreenerFailedAttempt | null;
  is_stale: boolean;
  results: ScreenerResult[];
}

export interface ScreenerRefreshSchedule {
  should_refresh_now: boolean;
  next_refresh_at: string;
}

export interface ScreenerWatchlistAddResult {
  added: boolean;
  already_present: boolean;
}

export const ACCOUNT_TYPES = [
  { label: "现金", value: "cash", icon: "cash" },
  { label: "银行卡", value: "bank", icon: "bank" },
  { label: "支付宝", value: "alipay", icon: "alipay" },
  { label: "微信", value: "wechat", icon: "wechat" },
  { label: "证券账户", value: "broker", icon: "broker" },
] as const;
