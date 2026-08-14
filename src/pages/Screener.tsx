import { useEffect, useRef, useState } from "react";
import { Alert, App, Button, Card, Modal, Space, Table, Tag, Tooltip, Typography } from "antd";
import { LineChartOutlined, PlusOutlined, SyncOutlined } from "@ant-design/icons";
import { api, formatInvokeError, formatMoney } from "../api";
import KlineChart from "../components/KlineChart";
import EmptyPlaceholder from "../components/layout/EmptyPlaceholder";
import PageHeader from "../components/layout/PageHeader";
import PageLoader from "../components/layout/PageLoader";
import type { Holding, ScreenerDashboard, ScreenerResult } from "../types";
import {
  defaultScreenerSort,
  formatCnyBillion,
  formatNullableNumber,
  formatPercentValue,
  screenerState,
  shouldShowRefreshSuccess,
  smallestDistance,
} from "./screenerView";

function formatDateTime(value: string | null | undefined) {
  if (!value) return "--";
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) return value;
  return parsed.toLocaleString("zh-CN", { hour12: false });
}

function resultToHolding(result: ScreenerResult): Holding {
  return {
    id: 0,
    code: result.code,
    name: result.name,
    type: "stock",
    quantity: 0,
    cost_price: result.current_price,
    current_price: result.current_price,
    market: "cn",
    created_at: result.price_observed_at,
    updated_at: result.price_observed_at,
    market_value: 0,
    cost_value: 0,
    profit: 0,
    profit_rate: 0,
  };
}

export default function ScreenerPage() {
  const { message } = App.useApp();
  const [dashboard, setDashboard] = useState<ScreenerDashboard | null>(null);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [klineResult, setKlineResult] = useState<ScreenerResult | null>(null);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const mountedRef = useRef(false);

  const clearSchedule = () => {
    if (timerRef.current) clearTimeout(timerRef.current);
    timerRef.current = null;
  };

  const refresh = async (showSuccess = false) => {
    setRefreshing(true);
    try {
      const next = await api.refreshScreener();
      if (!mountedRef.current) return;
      setDashboard(next);
      setError(null);
      if (showSuccess) {
        if (shouldShowRefreshSuccess(next)) {
          message.success("选股结果已刷新");
        } else {
          message.warning(next.latest_failed_attempt?.failure_summary?.message ?? "刷新未生成新的选股结果");
        }
      }
    } catch (nextError) {
      if (!mountedRef.current) return;
      setError(formatInvokeError(nextError));
    } finally {
      if (mountedRef.current) setRefreshing(false);
    }
  };

  const armSchedule = async () => {
    clearSchedule();
    try {
      const schedule = await api.getScreenerRefreshSchedule();
      if (!mountedRef.current) return;
      if (schedule.should_refresh_now) await refresh(false);
      const delay = Math.max(1_000, new Date(schedule.next_refresh_at).getTime() - Date.now());
      timerRef.current = setTimeout(() => void refresh(false).then(armSchedule), delay);
    } catch {
      timerRef.current = setTimeout(() => void armSchedule(), 60_000);
    }
  };

  useEffect(() => {
    mountedRef.current = true;
    const load = async () => {
      setLoading(true);
      try {
        const next = await api.getScreenerDashboard();
        if (!mountedRef.current) return;
        setDashboard(next);
        await armSchedule();
      } catch (nextError) {
        if (mountedRef.current) setError(formatInvokeError(nextError));
      } finally {
        if (mountedRef.current) setLoading(false);
      }
    };
    void load();
    return () => {
      mountedRef.current = false;
      clearSchedule();
    };
  }, []);

  if (loading) return <PageLoader tip="加载选股结果..." />;

  const state = dashboard ? screenerState(dashboard) : "first-use";
  const rows = defaultScreenerSort(dashboard?.results ?? []);
  const displayedRun = dashboard?.displayed_run;
  const failure = dashboard?.latest_failed_attempt?.failure_summary;

  return (
    <div>
      <PageHeader
        title="选股"
        subtitle="央企高股息与 BOLL 下轨共振筛选"
        actions={
          <Button icon={<SyncOutlined />} loading={refreshing} onClick={() => void refresh(true)}>
            手动刷新
          </Button>
        }
      />

      {error && <Alert type="error" showIcon style={{ marginBottom: 16 }} message={error} />}
      {failure && (
        <Alert
          type={state === "failed" ? "error" : "warning"}
          showIcon
          style={{ marginBottom: 16 }}
          message={`最近刷新失败：${failure.message}`}
        />
      )}

      <Card className="stat-card" style={{ marginBottom: 16 }}>
        <div className="screener-status-grid">
          <div><Typography.Text type="secondary">匹配数</Typography.Text><Typography.Text strong>{rows.length}</Typography.Text></div>
          <div><Typography.Text type="secondary">状态</Typography.Text><Tag bordered={false} color={state === "stale" ? "orange" : state === "failed" ? "red" : "blue"}>{refreshing ? "刷新中" : state}</Tag></div>
          <div><Typography.Text type="secondary">最近完成</Typography.Text><Typography.Text strong>{formatDateTime(displayedRun?.completed_at)}</Typography.Text></div>
          <div><Typography.Text type="secondary">跳过</Typography.Text><Typography.Text strong>{displayedRun?.skipped_count ?? 0}</Typography.Text></div>
        </div>
        <div className="screener-rule-list">
          <span>中央企业实际控制人</span><span>总市值 ≥ 500 亿</span><span>上年现金股息率 ≥ 5%</span><span>日线或周线 BOLL 下轨 2% 内</span>
        </div>
      </Card>

      {rows.length === 0 ? (
        <Card className="stat-card"><EmptyPlaceholder description={state === "failed" ? "暂无可展示缓存，刷新失败后仍没有历史结果" : "当前没有符合固定策略的标的"} /></Card>
      ) : (
        <Card className="stat-card screener-table">
          <Table<ScreenerResult>
            rowKey={(row) => `${row.exchange}:${row.code}`}
            size="small"
            dataSource={rows}
            scroll={{ x: 1480 }}
            pagination={{ pageSize: 20, showSizeChanger: false }}
            columns={[
              { title: "标的", fixed: "left", width: 180, render: (_, row) => <span>{row.name}<br /><Typography.Text type="secondary">{row.code} · {row.exchange}</Typography.Text></span> },
              { title: "市值", dataIndex: "market_cap_cny", sorter: (a, b) => a.market_cap_cny - b.market_cap_cny, render: formatCnyBillion },
              { title: "每股分红", dataIndex: "cash_dividend_per_share", render: (v: number) => formatMoney(v) },
              { title: "股息率", dataIndex: "dividend_yield", sorter: (a, b) => a.dividend_yield - b.dividend_yield, render: formatPercentValue },
              { title: "现价", dataIndex: "current_price", render: (v: number) => formatMoney(v) },
              { title: "日下轨", dataIndex: "daily_lower_band", render: (v: number | null) => formatNullableNumber(v) },
              { title: "日距离", dataIndex: "daily_distance", render: (v: number | null) => (v === null ? "--" : formatPercentValue(v)) },
              { title: "周下轨", dataIndex: "weekly_lower_band", render: (v: number | null) => formatNullableNumber(v) },
              { title: "周距离", dataIndex: "weekly_distance", render: (v: number | null) => (v === null ? "--" : formatPercentValue(v)) },
              { title: "默认距离", sorter: (a, b) => smallestDistance(a) - smallestDistance(b), render: (_, row) => formatPercentValue(smallestDistance(row)) },
              { title: "匹配", render: (_, row) => <Space>{row.matched_periods.map((period) => <Tag key={period}>{period}</Tag>)}</Space> },
              { title: "时间", dataIndex: "price_observed_at", render: formatDateTime },
              { title: "操作", fixed: "right", width: 96, render: (_, row) => <Space><Tooltip title="查看K线"><Button type="text" icon={<LineChartOutlined />} onClick={() => setKlineResult(row)} /></Tooltip><Tooltip title="加入量化关注"><Button type="text" icon={<PlusOutlined />} onClick={async () => { const result = await api.addScreenerResultToWatchlist(row.code, row.exchange, row.name); message.success(result.added ? "已加入量化关注" : "已在量化关注列表中"); }} /></Tooltip></Space> },
            ]}
          />
        </Card>
      )}

      <Modal open={!!klineResult} onCancel={() => setKlineResult(null)} footer={null} width={960} destroyOnClose title={klineResult ? `${klineResult.name}（${klineResult.code}）K线` : "K线"}>
        {klineResult && <KlineChart holding={resultToHolding(klineResult)} />}
      </Modal>
    </div>
  );
}
