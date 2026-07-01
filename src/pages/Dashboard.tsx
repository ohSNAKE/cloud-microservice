import { useCallback, useEffect, useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";
import { Button, Card, Col, Empty, Row, Spin, Statistic, Typography } from "antd";
import {
  ArrowDownOutlined,
  ArrowUpOutlined,
  LineChartOutlined,
  PlusOutlined,
  WalletOutlined,
} from "@ant-design/icons";
import ReactECharts from "echarts-for-react";
import { api, formatMoney } from "../api";
import type { DashboardSummary, MonthlyStat } from "../types";

export default function DashboardPage() {
  const navigate = useNavigate();
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
    const onFocus = () => loadData();
    window.addEventListener("focus", onFocus);
    return () => window.removeEventListener("focus", onFocus);
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
            { name: "可动用资产", value: summary.liquid_assets },
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

  if (summary.is_empty) {
    return (
      <div>
        <div className="page-header">
          <h2>财务总览</h2>
          <p>从这里开始管理你的资产和支出</p>
        </div>
        <Card>
          <Empty description="还没有任何数据，3 步快速开始">
            <div style={{ marginTop: 16, display: "flex", gap: 12, justifyContent: "center", flexWrap: "wrap" }}>
              <Button type="primary" icon={<PlusOutlined />} onClick={() => navigate("/transactions")}>
                记第一笔账
              </Button>
              <Button icon={<LineChartOutlined />} onClick={() => navigate("/holdings")}>
                添加投资持仓
              </Button>
            </div>
            <Typography.Paragraph type="secondary" style={{ marginTop: 24, marginBottom: 0 }}>
              所有数据保存在本地，可在「设置」中备份导出
            </Typography.Paragraph>
          </Empty>
        </Card>
      </div>
    );
  }

  return (
    <div>
      <div className="page-header" style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start" }}>
        <div>
          <h2>财务总览</h2>
          <p>可动用资产 + 投资市值 = 真实净资产（证券账户余额不计入，避免重复）</p>
        </div>
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
          <Button icon={<ArrowDownOutlined />} onClick={() => navigate("/transactions?action=expense")}>
            记支出
          </Button>
          <Button icon={<ArrowUpOutlined />} onClick={() => navigate("/transactions?action=income")}>
            记收入
          </Button>
          <Button type="primary" icon={<WalletOutlined />} onClick={() => navigate("/holdings")}>
            看持仓
          </Button>
        </div>
      </div>

      <Row gutter={[16, 16]}>
        <Col xs={24} sm={12} lg={6}>
          <Card className="stat-card" hoverable onClick={() => navigate("/")}>
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
            title="本月收支"
            extra={<Button type="link" onClick={() => navigate("/transactions")}>查看明细</Button>}
          >
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
            {summary.broker_balance > 0 && (
              <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                证券账户余额 {formatMoney(summary.broker_balance)} 已排除在总资产外
              </Typography.Text>
            )}
          </Card>
        </Col>
        <Col xs={24} lg={8}>
          <Card
            title="本月概览"
            extra={<Button type="link" onClick={() => navigate("/reports")}>报表分析</Button>}
          >
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
