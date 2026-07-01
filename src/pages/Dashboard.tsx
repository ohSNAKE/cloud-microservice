import { useCallback, useEffect, useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";
import { Button, Card, Col, Row, Statistic, Typography, Alert } from "antd";
import {
  ArrowDownOutlined,
  ArrowUpOutlined,
  LineChartOutlined,
  PlusOutlined,
  WalletOutlined,
} from "@ant-design/icons";
import ReactECharts from "echarts-for-react";
import { api, formatMoney } from "../api";
import EmptyPlaceholder from "../components/layout/EmptyPlaceholder";
import PageHeader from "../components/layout/PageHeader";
import PageLoader from "../components/layout/PageLoader";
import { chartGrid } from "../constants/chartTheme";
import { useQuickAdd } from "../context/QuickAddContext";
import { useTheme } from "../context/ThemeContext";
import type { BudgetAlert, DashboardSummary, MonthlyStat } from "../types";

export default function DashboardPage() {
  const navigate = useNavigate();
  const { openQuickAdd } = useQuickAdd();
  const { chartColors } = useTheme();
  const [loading, setLoading] = useState(true);
  const [summary, setSummary] = useState<DashboardSummary | null>(null);
  const [monthlyStats, setMonthlyStats] = useState<MonthlyStat[]>([]);
  const [budgetAlerts, setBudgetAlerts] = useState<BudgetAlert[]>([]);

  const loadData = useCallback(async () => {
    setLoading(true);
    try {
      const [dashboard, monthly, alerts] = await Promise.all([
        api.getDashboard(),
        api.getMonthlyStats(6),
        api.getBudgetAlerts(),
      ]);
      setSummary(dashboard);
      setMonthlyStats(monthly);
      setBudgetAlerts(alerts);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadData();
    const onFocus = () => loadData();
    window.addEventListener("focus", onFocus);
    return () => window.removeEventListener("focus", onFocus);
  }, [loadData]);

  const trendOption = useMemo(() => {
    const months = monthlyStats.map((item) => item.month);
    return {
      tooltip: { trigger: "axis" },
      legend: { data: ["收入", "支出"] },
      grid: chartGrid,
      xAxis: { type: "category", data: months },
      yAxis: { type: "value" },
      series: [
        {
          name: "收入",
          type: "bar",
          data: monthlyStats.map((item) => item.income),
          itemStyle: { color: chartColors.income, borderRadius: [4, 4, 0, 0] },
        },
        {
          name: "支出",
          type: "bar",
          data: monthlyStats.map((item) => item.expense),
          itemStyle: { color: chartColors.expense, borderRadius: [4, 4, 0, 0] },
        },
      ],
    };
  }, [monthlyStats, chartColors]);

  const assetOption = useMemo(() => {
    if (!summary) return {};
    return {
      tooltip: { trigger: "item" },
      color: [chartColors.primary, "#69b1ff"],
      series: [
        {
          type: "pie",
          radius: ["42%", "70%"],
          data: [
            { name: "可动用资产", value: summary.liquid_assets },
            { name: "投资市值", value: summary.holding_value },
          ],
        },
      ],
    };
  }, [summary, chartColors]);

  if (loading || !summary) {
    return <PageLoader tip="加载财务数据..." />;
  }

  if (summary.is_empty) {
    return (
      <div>
        <PageHeader title="欢迎使用财记" subtitle="3 步开始管理你的资产与支出" />
        <Card className="stat-card">
          <EmptyPlaceholder description="还没有任何数据">
            <div style={{ display: "flex", gap: 12, justifyContent: "center", flexWrap: "wrap" }}>
              <Button type="primary" icon={<PlusOutlined />} onClick={openQuickAdd}>
                智能记第一笔
              </Button>
              <Button icon={<LineChartOutlined />} onClick={() => navigate("/holdings")}>
                添加投资持仓
              </Button>
            </div>
            <Typography.Paragraph type="secondary" style={{ marginTop: 24, marginBottom: 0 }}>
              所有数据保存在本地，可在「设置」中备份导出
            </Typography.Paragraph>
          </EmptyPlaceholder>
        </Card>
      </div>
    );
  }

  return (
    <div>
      <PageHeader
        subtitle="可动用资产 + 投资市值 = 真实净资产"
        actions={
          <>
            <Button icon={<ArrowDownOutlined />} onClick={openQuickAdd}>
              记一笔
            </Button>
            <Button icon={<ArrowUpOutlined />} onClick={() => navigate("/transactions?action=income")}>
              记收入
            </Button>
            <Button type="primary" icon={<WalletOutlined />} onClick={() => navigate("/holdings")}>
              看持仓
            </Button>
          </>
        }
      />

      {budgetAlerts.length > 0 && (
        <Alert
          type="warning"
          showIcon
          style={{ marginBottom: 16 }}
          message="预算超支提醒"
          description={
            <ul style={{ margin: 0, paddingLeft: 20 }}>
              {budgetAlerts.map((a) => (
                <li key={a.category_name}>
                  {a.category_icon} {a.category_name}：已用 {formatMoney(a.spent)} / 预算{" "}
                  {formatMoney(a.budget)}，超出 {formatMoney(a.over_amount)}
                </li>
              ))}
            </ul>
          }
          action={
            <Button size="small" onClick={() => navigate("/reports")}>
              查看预算
            </Button>
          }
        />
      )}

      <Row gutter={[16, 16]}>
        <Col xs={24} sm={12} lg={6}>
          <Card className="stat-card" hoverable>
            <div className="label">总资产</div>
            <div className="value">{formatMoney(summary.total_assets)}</div>
          </Card>
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <Card className="stat-card">
            <div className="label">可动用资产</div>
            <div className="value">{formatMoney(summary.liquid_assets)}</div>
          </Card>
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <Card className="stat-card" hoverable onClick={() => navigate("/holdings")}>
            <div className="label">投资市值</div>
            <div className="value">{formatMoney(summary.holding_value)}</div>
          </Card>
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <Card className="stat-card">
            <div className="label">持仓盈亏</div>
            <div className={`value ${summary.holding_profit >= 0 ? "profit-positive" : "profit-negative"}`}>
              {formatMoney(summary.holding_profit)}
            </div>
          </Card>
        </Col>
      </Row>

      <Row gutter={[16, 16]} style={{ marginTop: 16 }}>
        <Col xs={24} lg={8}>
          <Card
            className="stat-card"
            title="本月收支"
            extra={<Button type="link" onClick={() => navigate("/transactions")}>明细</Button>}
          >
            <Statistic title="收入" value={summary.month_income} precision={2} prefix="¥" valueStyle={{ color: chartColors.income }} />
            <Statistic
              title="支出"
              value={summary.month_expense}
              precision={2}
              prefix="¥"
              valueStyle={{ color: chartColors.expense }}
              style={{ marginTop: 16 }}
            />
            <Typography.Paragraph style={{ marginTop: 16, marginBottom: 0 }}>
              结余：<strong>{formatMoney(summary.month_balance)}</strong>
            </Typography.Paragraph>
          </Card>
        </Col>
        <Col xs={24} lg={8}>
          <Card className="stat-card" title="资产分布">
            <ReactECharts option={assetOption} style={{ height: 260 }} />
            {summary.broker_balance > 0 && (
              <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                证券账户余额 {formatMoney(summary.broker_balance)} 已排除在总资产外
              </Typography.Text>
            )}
          </Card>
        </Col>
        <Col xs={24} lg={8}>
          <Card
            className="stat-card"
            title="本月概览"
            extra={<Button type="link" onClick={() => navigate("/reports")}>报表</Button>}
          >
            <Statistic title="持仓数量" value={summary.holding_count} suffix="个" />
            <Statistic title="本月记账" value={summary.transaction_count} suffix="笔" style={{ marginTop: 16 }} />
          </Card>
        </Col>
      </Row>

      <Card className="stat-card" title="近 6 个月收支趋势" style={{ marginTop: 16 }}>
        <ReactECharts option={trendOption} style={{ height: 320 }} />
      </Card>
    </div>
  );
}
