import { useCallback, useEffect, useRef, useState } from "react";
import {
  Alert,
  App,
  Button,
  Card,
  Form,
  Input,
  Popconfirm,
  Select,
  Space,
  Switch,
  Table,
  Tag,
  Tooltip,
  Typography,
} from "antd";
import {
  BellOutlined,
  DeleteOutlined,
  PlusOutlined,
  SyncOutlined,
} from "@ant-design/icons";
import { api, formatInvokeError, formatMoney } from "../api";
import EmptyPlaceholder from "../components/layout/EmptyPlaceholder";
import PageHeader from "../components/layout/PageHeader";
import PageLoader from "../components/layout/PageLoader";
import { notifyGeneratedQuantSignals } from "../utils/quantNotifications";
import type {
  NewQuantWatchlistItem,
  QuantDashboard,
  QuantDirection,
  QuantOutputState,
  QuantSignal,
  QuantSignalFilter,
  QuantTarget,
  QuantTriggerZone,
  QuantWatchlistItem,
} from "../types";

const REFRESH_INTERVAL_MS = 60_000;
const DEFAULT_SIGNAL_FILTER: QuantSignalFilter = { limit: 100 };

const outputLabels: Record<QuantOutputState, string> = {
  buy_attention: "买入关注",
  sell_attention: "卖出关注",
  watch: "观望",
  quote_error: "行情异常",
};

const outputClasses: Record<QuantOutputState, string> = {
  buy_attention: "quant-signal-buy",
  sell_attention: "quant-signal-sell",
  watch: "quant-signal-watch",
  quote_error: "quant-signal-error",
};

const directionLabels: Record<QuantDirection, string> = {
  buy_attention: "买入关注",
  sell_attention: "卖出关注",
};

const triggerZoneLabels: Record<QuantTriggerZone, string> = {
  buy_1: "买一区",
  buy_2: "买二区",
  sell_1: "卖一区",
  sell_2: "卖二区",
};

const trendLabels: Record<QuantTarget["trend_state"], string> = {
  bullish: "多头",
  bearish: "空头",
  neutral: "中性",
  insufficient_data: "数据不足",
};

interface WatchlistFormValues {
  code: string;
  name?: string;
  market: string;
}

type CanApplyState = () => boolean;

function isSignalFilterActive(filter: QuantSignalFilter) {
  return Boolean(filter.code?.trim() || filter.direction || filter.from || filter.to);
}

function formatDateTime(value: string | null | undefined) {
  if (!value) return "--";
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) return value;
  return parsed.toLocaleString("zh-CN", { hour12: false });
}

function displayPrice(value: number | null) {
  return value === null ? "--" : formatMoney(value);
}

function sourceLabel(source: QuantSignal["source"]) {
  return source === "auto_grid" ? "自动网格" : source;
}

export default function QuantAlertsPage() {
  const { message } = App.useApp();
  const [dashboard, setDashboard] = useState<QuantDashboard | null>(null);
  const [watchlist, setWatchlist] = useState<QuantWatchlistItem[]>([]);
  const [signals, setSignals] = useState<QuantSignal[]>([]);
  const [signalFilter, setSignalFilter] = useState<QuantSignalFilter>(DEFAULT_SIGNAL_FILTER);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [historyLoading, setHistoryLoading] = useState(false);
  const [notificationWarning, setNotificationWarning] = useState<string | null>(null);
  const [addingWatchlist, setAddingWatchlist] = useState(false);
  const [updatingTargetKey, setUpdatingTargetKey] = useState<string | null>(null);
  const [codeFilterInput, setCodeFilterInput] = useState("");
  const [nextRefreshAt, setNextRefreshAt] = useState<Date | null>(null);
  const [addForm] = Form.useForm<WatchlistFormValues>();
  const mountedRef = useRef(false);
  const refreshInFlightRef = useRef(false);
  const signalFilterRef = useRef<QuantSignalFilter>(DEFAULT_SIGNAL_FILTER);
  const signalFilterActiveRef = useRef(false);

  const loadDashboardAndWatchlist = useCallback(async (canApply: CanApplyState = () => mountedRef.current) => {
    const [nextDashboard, nextWatchlist] = await Promise.all([
      api.getQuantDashboard(),
      api.listQuantWatchlist(),
    ]);

    if (!canApply()) return;
    setDashboard(nextDashboard);
    setWatchlist(nextWatchlist);
    setSignals(nextDashboard.recent_signals);
  }, []);

  const scheduleNextRefresh = useCallback(() => {
    if (!mountedRef.current) return;
    setNextRefreshAt(new Date(Date.now() + REFRESH_INTERVAL_MS));
  }, []);

  const refreshQuantSignals = useCallback(
    async (showSuccess = false, canApply: CanApplyState = () => mountedRef.current) => {
      if (refreshInFlightRef.current) return;
      refreshInFlightRef.current = true;
      if (canApply()) setRefreshing(true);

      try {
        const result = await api.refreshQuantSignals();
        if (!canApply()) return;

        setDashboard(result.dashboard);

        if (signalFilterActiveRef.current) {
          setHistoryLoading(true);
          const filteredSignals = await api.listQuantSignals(signalFilterRef.current);
          if (!canApply()) return;
          setSignals(filteredSignals);
        } else {
          setSignals(result.dashboard.recent_signals);
        }

        const notificationResult = await notifyGeneratedQuantSignals(result.generated_signals);
        if (!canApply()) return;
        if (notificationResult.denied) {
          setNotificationWarning("桌面通知权限未开启，本次量化提醒已跳过。");
        } else {
          setNotificationWarning(null);
        }

        scheduleNextRefresh();
        if (showSuccess) message.success("量化信号已刷新");
      } catch (error) {
        if (canApply()) message.error(formatInvokeError(error));
      } finally {
        refreshInFlightRef.current = false;
        if (canApply()) {
          setRefreshing(false);
          setHistoryLoading(false);
        }
      }
    },
    [message, scheduleNextRefresh],
  );

  useEffect(() => {
    mountedRef.current = true;
    let cancelled = false;
    let intervalId: ReturnType<typeof setInterval> | undefined;

    const isActiveEffect = () => mountedRef.current && !cancelled;

    const loadInitialData = async () => {
      setLoading(true);
      try {
        await loadDashboardAndWatchlist(isActiveEffect);
        if (!isActiveEffect()) return;

        await refreshQuantSignals(false, isActiveEffect);
        if (!isActiveEffect()) return;

        scheduleNextRefresh();
        intervalId = setInterval(() => {
          void refreshQuantSignals();
        }, REFRESH_INTERVAL_MS);
      } catch (error) {
        if (isActiveEffect()) message.error(formatInvokeError(error));
      } finally {
        if (isActiveEffect()) setLoading(false);
      }
    };

    void loadInitialData();

    return () => {
      cancelled = true;
      mountedRef.current = false;
      if (intervalId) clearInterval(intervalId);
    };
  }, [loadDashboardAndWatchlist, message, refreshQuantSignals, scheduleNextRefresh]);

  const loadSignalsWithFilter = async (nextFilter: QuantSignalFilter) => {
    signalFilterRef.current = nextFilter;
    signalFilterActiveRef.current = isSignalFilterActive(nextFilter);
    setSignalFilter(nextFilter);
    setHistoryLoading(true);

    try {
      const filteredSignals = await api.listQuantSignals(nextFilter);
      if (mountedRef.current) setSignals(filteredSignals);
    } catch (error) {
      message.error(formatInvokeError(error));
    } finally {
      if (mountedRef.current) setHistoryLoading(false);
    }
  };

  const updateTargetSettings = async (
    target: QuantTarget,
    nextSettings: { enabled: boolean; desktop_notification_enabled: boolean },
  ) => {
    const targetKey = `${target.market}:${target.code}`;
    setUpdatingTargetKey(targetKey);
    try {
      const updatedTarget = await api.updateQuantStrategySettings(
        target.code,
        target.market,
        nextSettings,
      );
      setDashboard((current) =>
        current
          ? {
              ...current,
              targets: current.targets.map((item) =>
                item.code === updatedTarget.code && item.market === updatedTarget.market
                  ? updatedTarget
                  : item,
              ),
            }
          : current,
      );
      setWatchlist((current) =>
        current.map((item) =>
          item.code === updatedTarget.code && item.market === updatedTarget.market
            ? { ...item, enabled: updatedTarget.enabled }
            : item,
        ),
      );
      message.success("策略设置已更新");
    } catch (error) {
      message.error(formatInvokeError(error));
    } finally {
      if (mountedRef.current) setUpdatingTargetKey(null);
    }
  };

  const handleAddWatchlist = async () => {
    const values = await addForm.validateFields();
    const input: NewQuantWatchlistItem = {
      code: values.code.trim(),
      name: values.name?.trim() || undefined,
      market: values.market || "cn",
      enabled: true,
    };

    setAddingWatchlist(true);
    try {
      await api.addQuantWatchlist(input);
      message.success("关注标的已添加");
      addForm.resetFields();
      await loadDashboardAndWatchlist();
    } catch (error) {
      message.error(formatInvokeError(error));
    } finally {
      if (mountedRef.current) setAddingWatchlist(false);
    }
  };

  const handleDeleteWatchlist = async (item: QuantWatchlistItem) => {
    try {
      await api.deleteQuantWatchlist(item.id);
      message.success("关注标的已删除");
      await loadDashboardAndWatchlist();
    } catch (error) {
      message.error(formatInvokeError(error));
    }
  };

  const handleCodeFilter = (value: string) => {
    const code = value.trim();
    setCodeFilterInput(code);
    void loadSignalsWithFilter({
      ...signalFilterRef.current,
      code: code || undefined,
      limit: 100,
    });
  };

  const handleDirectionFilter = (value: "all" | QuantDirection) => {
    void loadSignalsWithFilter({
      ...signalFilterRef.current,
      direction: value === "all" ? undefined : value,
      limit: 100,
    });
  };

  if (loading) {
    return <PageLoader tip="加载量化提醒..." />;
  }

  const targets = dashboard?.targets ?? [];
  const nextRefreshText = dashboard?.next_refresh_at
    ? formatDateTime(dashboard.next_refresh_at)
    : formatDateTime(nextRefreshAt?.toISOString());

  return (
    <div>
      <PageHeader
        title="量化提醒"
        subtitle="自动网格信号、关注标的与桌面提醒"
        actions={
          <Button
            icon={<SyncOutlined />}
            loading={refreshing}
            onClick={() => void refreshQuantSignals(true)}
          >
            刷新信号
          </Button>
        }
      />

      {notificationWarning && (
        <Alert
          type="warning"
          showIcon
          closable
          style={{ marginBottom: 16 }}
          message={notificationWarning}
          onClose={() => setNotificationWarning(null)}
        />
      )}

      <Card className="stat-card" style={{ marginBottom: 16 }}>
        <div className="quant-status-grid">
          <div>
            <Typography.Text type="secondary">交易时段</Typography.Text>
            <div>
              <Tag color={dashboard?.is_trading_time ? "green" : "default"} bordered={false}>
                {dashboard?.is_trading_time ? "盘中" : "非交易时段"}
              </Tag>
            </div>
          </div>
          <div>
            <Typography.Text type="secondary">轮询状态</Typography.Text>
            <div>
              <Tag color={refreshing ? "processing" : "blue"} bordered={false}>
                {refreshing ? "刷新中" : "60 秒轮询"}
              </Tag>
            </div>
          </div>
          <div>
            <Typography.Text type="secondary">下次刷新</Typography.Text>
            <Typography.Text strong>{nextRefreshText}</Typography.Text>
          </div>
          <div>
            <Typography.Text type="secondary">风险提示</Typography.Text>
            <Typography.Text strong>信号仅供参考，不构成投资建议</Typography.Text>
          </div>
        </div>
      </Card>

      {targets.length === 0 ? (
        <Card className="stat-card" style={{ marginBottom: 16 }}>
          <EmptyPlaceholder description="暂无量化标的，添加关注后开始追踪信号" />
        </Card>
      ) : (
        <div className="quant-target-grid" style={{ marginBottom: 16 }}>
          {targets.map((target) => {
            const targetKey = `${target.market}:${target.code}`;
            return (
              <Card key={targetKey} className="stat-card quant-target-card" hoverable>
                <div className="quant-target-card__header">
                  <div>
                    <Typography.Text
                      strong
                      className="quant-target-card__name"
                      ellipsis={{ tooltip: target.name || target.code }}
                    >
                      {target.name || target.code}
                    </Typography.Text>
                    <span className="quant-target-card__code">
                      {target.code} · {target.market}
                    </span>
                  </div>
                  <Tag bordered={false} className={outputClasses[target.output_state]}>
                    {outputLabels[target.output_state]}
                  </Tag>
                </div>

                <div className="quant-target-card__price">{displayPrice(target.current_price)}</div>

                <div className="quant-target-card__meta">
                  <span>趋势：{trendLabels[target.trend_state]}</span>
                  <span>触发区：{target.current_trigger_zone ? triggerZoneLabels[target.current_trigger_zone] : "--"}</span>
                  <span>触发：{formatDateTime(target.latest_signal?.triggered_at)}</span>
                  <span>行情：{formatDateTime(target.quote_fetched_at)}</span>
                </div>

                {target.last_error && (
                  <Alert
                    type="warning"
                    showIcon
                    message={target.last_error}
                    style={{ marginTop: 12 }}
                  />
                )}

                <Space size="large" wrap style={{ marginTop: 14 }}>
                  <Space size={6}>
                    <Typography.Text type="secondary">策略</Typography.Text>
                    <Switch
                      size="small"
                      checked={target.enabled}
                      loading={updatingTargetKey === targetKey}
                      onChange={(next) =>
                        void updateTargetSettings(target, {
                          enabled: next,
                          desktop_notification_enabled: target.desktop_notification_enabled,
                        })
                      }
                    />
                  </Space>
                  <Space size={6}>
                    <BellOutlined />
                    <Typography.Text type="secondary">桌面提醒</Typography.Text>
                    <Switch
                      size="small"
                      checked={target.desktop_notification_enabled}
                      loading={updatingTargetKey === targetKey}
                      onChange={(next) =>
                        void updateTargetSettings(target, {
                          enabled: target.enabled,
                          desktop_notification_enabled: next,
                        })
                      }
                    />
                  </Space>
                </Space>
              </Card>
            );
          })}
        </div>
      )}

      <Card className="stat-card" title="关注列表" style={{ marginBottom: 16 }}>
        <Form
          form={addForm}
          layout="inline"
          className="filter-bar"
          initialValues={{ market: "cn" }}
          onFinish={() => void handleAddWatchlist()}
        >
          <Form.Item name="code" rules={[{ required: true, message: "输入代码" }]}>
            <Input placeholder="代码" style={{ width: 140 }} />
          </Form.Item>
          <Form.Item name="name">
            <Input placeholder="名称" style={{ width: 160 }} />
          </Form.Item>
          <Form.Item name="market" rules={[{ required: true, message: "选择市场" }]}>
            <Select
              style={{ width: 120 }}
              options={[
                { label: "A股", value: "cn" },
              ]}
            />
          </Form.Item>
          <Button type="primary" htmlType="submit" icon={<PlusOutlined />} loading={addingWatchlist}>
            添加关注
          </Button>
        </Form>

        <Table<QuantWatchlistItem>
          rowKey="id"
          size="small"
          dataSource={watchlist}
          pagination={false}
          locale={{ emptyText: "暂无关注标的" }}
          columns={[
            { title: "名称", dataIndex: "name", render: (value: string, item) => value || item.code },
            { title: "代码", dataIndex: "code" },
            { title: "市场", dataIndex: "market" },
            {
              title: "状态",
              dataIndex: "enabled",
              render: (enabled: boolean) => (
                <Tag color={enabled ? "green" : "default"} bordered={false}>
                  {enabled ? "已启用" : "已停用"}
                </Tag>
              ),
            },
            {
              title: "操作",
              render: (_, item) => (
                <Popconfirm
                  title={`删除「${item.name || item.code}」？`}
                  okText="删除"
                  cancelText="取消"
                  onConfirm={() => void handleDeleteWatchlist(item)}
                >
                  <Tooltip title="删除">
                    <Button type="text" danger size="small" icon={<DeleteOutlined />} />
                  </Tooltip>
                </Popconfirm>
              ),
            },
          ]}
        />
      </Card>

      <Card className="stat-card" title="信号历史">
        <div className="filter-bar">
          <Input.Search
            allowClear
            placeholder="按代码筛选"
            value={codeFilterInput}
            onChange={(event) => {
              setCodeFilterInput(event.target.value);
              if (!event.target.value) handleCodeFilter("");
            }}
            onSearch={handleCodeFilter}
            style={{ width: 200 }}
          />
          <Select<"all" | QuantDirection>
            value={signalFilter.direction ?? "all"}
            style={{ width: 140 }}
            onChange={handleDirectionFilter}
            options={[
              { label: "全部", value: "all" },
              { label: "买入关注", value: "buy_attention" },
              { label: "卖出关注", value: "sell_attention" },
            ]}
          />
        </div>

        <Table<QuantSignal>
          rowKey="id"
          loading={historyLoading}
          dataSource={signals}
          pagination={{ pageSize: 10, showSizeChanger: false }}
          locale={{ emptyText: "暂无信号记录" }}
          columns={[
            { title: "时间", dataIndex: "triggered_at", render: (value: string) => formatDateTime(value) },
            {
              title: "股票",
              render: (_, item) => `${item.name || item.code}（${item.code}）`,
            },
            {
              title: "方向",
              dataIndex: "direction",
              render: (value: QuantDirection) => (
                <Tag
                  bordered={false}
                  className={
                    value === "buy_attention" ? "quant-signal-buy" : "quant-signal-sell"
                  }
                >
                  {directionLabels[value]}
                </Tag>
              ),
            },
            { title: "触发价", dataIndex: "trigger_price", render: (value: number) => formatMoney(value) },
            { title: "来源", dataIndex: "source", render: sourceLabel },
            {
              title: "触发区",
              dataIndex: "trigger_zone",
              render: (value: QuantTriggerZone) => triggerZoneLabels[value],
            },
          ]}
        />
      </Card>
    </div>
  );
}
