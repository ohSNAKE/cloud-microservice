import { useCallback, useEffect, useMemo, useState } from "react";
import { Card, Col, Row, Spin, Statistic, Typography } from "antd";
import ReactECharts from "echarts-for-react";
import { api, formatMoney } from "../api";
import type { DashboardSummary, MonthlyStat } from "../types";

export default function DashboardPage() {
  const [loading, setLoading] = useState(true);
  const [summary, setSummary] = useState<DashboardSummary | null>(null);
  const [monthlyStats, setMonthlyStats] = useState<MonthlyStat[]>([]);

  const loadData = useCallback(async () => {
    setLoading(true);
    try {
      const [dashboard, monthly] = await Promise.all([
        api.getDashboard(),
        api.getMonthlyStats(6),
      ]);
      setSummary(dashboard);
      setMonthlyStats(monthly);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadData();
  }, [loadData]);

  const trendOption = useMemo(() => {
    const months = monthlyStats.map((item) => item.month);
    return {
      tooltip: { trigger: "axis" },
      legend: { data: ["收入", "支出"] },
      grid: { left: 40, right: 20, top: 40, bottom: 30 },
      xAxis: { type: "category", data: months },
      yAxis: { type: "value" },
      series: [
        {
          name: "收入",
          type: "bar",
          data: monthlyStats.map((item) => item.income),
          itemStyle: { color: "#52c41a" },
        },
        {
          name: "支出",
          type: "bar",
          data: monthlyStats.map((item) => item.expense),
          itemStyle: { color: "#ff4d4f" },
        },
      ],
    };
  }, [monthlyStats]);

  const assetOption = useMemo(() => {
    if (!summary) return {};
    return {
      tooltip: { trigger: "item" },
      series: [
        {
          type: "pie",
          radius: ["42%", "70%"],
          data: [
            { name: "账户余额", value: summary.account_balance },
            { name: "投资市值", value: summary.holding_value },
          ],
        },
      ],
    };
  }, [summary]);

  if (loading || !summary) {
    return (
      <div style={{ textAlign: "center", padding: 80 }}>
        <Spin size="large" />
      </div>
    );
  }

  return (
    <div>
      <div className="page-header">
        <h2>财务总览</h2>
        <p>掌握净资产、本月收支与投资表现</p>
      </div>

      <Row gutter={[16, 16]}>
        <Col xs={24} sm={12} lg={6}>
          <Card className="stat-card">
            <div className="label">总资产</div>
            <div className="value">{formatMoney(summary.total_assets)}</div>
          </Card>
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <Card className="stat-card">
            <div className="label">账户余额</div>
            <div className="value">{formatMoney(summary.account_balance)}</div>
          </Card>
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <Card className="stat-card">
            <div className="label">持仓市值</div>
            <div className="value">{formatMoney(summary.holding_value)}</div>
          </Card>
        </Col>
        <Col xs={24} sm={12} lg={6}>
          <Card className="stat-card">
            <div className="label">持仓盈亏</div>
            <div
              className={`value ${summary.holding_profit >= 0 ? "profit-positive" : "profit-negative"}`}
            >
              {formatMoney(summary.holding_profit)}
            </div>
          </Card>
        </Col>
      </Row>

      <Row gutter={[16, 16]} style={{ marginTop: 16 }}>
        <Col xs={24} lg={8}>
          <Card title="本月收支">
            <Statistic title="收入" value={summary.month_income} precision={2} prefix="¥" />
            <Statistic
              title="支出"
              value={summary.month_expense}
              precision={2}
              prefix="¥"
              style={{ marginTop: 16 }}
            />
            <Typography.Paragraph style={{ marginTop: 16, marginBottom: 0 }}>
              结余：<strong>{formatMoney(summary.month_balance)}</strong>
            </Typography.Paragraph>
          </Card>
        </Col>
        <Col xs={24} lg={8}>
          <Card title="资产分布">
            <ReactECharts option={assetOption} style={{ height: 260 }} />
          </Card>
        </Col>
        <Col xs={24} lg={8}>
          <Card title="数据概览">
            <Statistic title="持仓数量" value={summary.holding_count} suffix="个" />
            <Statistic
              title="本月记账"
              value={summary.transaction_count}
              suffix="笔"
              style={{ marginTop: 16 }}
            />
          </Card>
        </Col>
      </Row>

      <Card title="近 6 个月收支趋势" style={{ marginTop: 16 }}>
        <ReactECharts option={trendOption} style={{ height: 320 }} />
      </Card>
    </div>
  );
}
