import { useCallback, useEffect, useMemo, useState } from "react";
import { Card, Col, DatePicker, Row, Spin, Table } from "antd";
import ReactECharts from "echarts-for-react";
import dayjs from "dayjs";
import { api, formatMoney } from "../api";
import type { CategoryStat, MonthlyStat } from "../types";

export default function ReportsPage() {
  const [loading, setLoading] = useState(true);
  const [month, setMonth] = useState(dayjs().format("YYYY-MM"));
  const [expenseStats, setExpenseStats] = useState<CategoryStat[]>([]);
  const [incomeStats, setIncomeStats] = useState<CategoryStat[]>([]);
  const [monthlyStats, setMonthlyStats] = useState<MonthlyStat[]>([]);

  const loadData = useCallback(async () => {
    setLoading(true);
    try {
      const [expense, income, monthly] = await Promise.all([
        api.getCategoryStats(month, "expense"),
        api.getCategoryStats(month, "income"),
        api.getMonthlyStats(12),
      ]);
      setExpenseStats(expense);
      setIncomeStats(income);
      setMonthlyStats(monthly);
    } finally {
      setLoading(false);
    }
  }, [month]);

  useEffect(() => {
    loadData();
  }, [loadData]);

  const expenseChartOption = useMemo(() => ({
    tooltip: { trigger: "item" },
    series: [{
      type: "pie",
      radius: "65%",
      data: expenseStats.map((s) => ({
        name: `${s.category_icon} ${s.category_name}`,
        value: s.amount,
      })),
    }],
  }), [expenseStats]);

  const trendOption = useMemo(() => ({
    tooltip: { trigger: "axis" },
    legend: { data: ["收入", "支出"] },
    xAxis: { type: "category", data: monthlyStats.map((m) => m.month) },
    yAxis: { type: "value" },
    series: [
      { name: "收入", type: "line", data: monthlyStats.map((m) => m.income), smooth: true, itemStyle: { color: "#52c41a" } },
      { name: "支出", type: "line", data: monthlyStats.map((m) => m.expense), smooth: true, itemStyle: { color: "#ff4d4f" } },
    ],
  }), [monthlyStats]);

  if (loading) {
    return <div style={{ textAlign: "center", padding: 80 }}><Spin size="large" /></div>;
  }

  const totalExpense = expenseStats.reduce((s, i) => s + i.amount, 0);
  const totalIncome = incomeStats.reduce((s, i) => s + i.amount, 0);

  return (
    <div>
      <div className="page-header">
        <h2>报表分析</h2>
        <p>从分类和趋势两个维度复盘你的财务状况</p>
      </div>

      <Card style={{ marginBottom: 16 }}>
        <DatePicker
          picker="month"
          value={dayjs(month)}
          onChange={(v) => setMonth(v?.format("YYYY-MM") ?? month)}
        />
        <span style={{ marginLeft: 24 }}>
          本月收入 {formatMoney(totalIncome)} · 支出 {formatMoney(totalExpense)} · 结余 {formatMoney(totalIncome - totalExpense)}
        </span>
      </Card>

      <Row gutter={[16, 16]}>
        <Col xs={24} lg={12}>
          <Card title="支出分类占比">
            {expenseStats.length > 0 ? (
              <>
                <ReactECharts option={expenseChartOption} style={{ height: 280 }} />
                <Table<CategoryStat>
                  size="small"
                  rowKey="category_id"
                  pagination={false}
                  dataSource={expenseStats}
                  columns={[
                    { title: "分类", render: (_, r) => `${r.category_icon} ${r.category_name}` },
                    { title: "金额", dataIndex: "amount", render: (v: number) => formatMoney(v) },
                    { title: "占比", dataIndex: "percentage", render: (v: number) => `${v.toFixed(1)}%` },
                  ]}
                />
              </>
            ) : (
              <div style={{ textAlign: "center", padding: 40, color: "#999" }}>本月暂无支出记录</div>
            )}
          </Card>
        </Col>
        <Col xs={24} lg={12}>
          <Card title="收入来源">
            <Table<CategoryStat>
              size="small"
              rowKey="category_id"
              pagination={false}
              dataSource={incomeStats}
              locale={{ emptyText: "本月暂无收入记录" }}
              columns={[
                { title: "分类", render: (_, r) => `${r.category_icon} ${r.category_name}` },
                { title: "金额", dataIndex: "amount", render: (v: number) => formatMoney(v) },
                { title: "占比", dataIndex: "percentage", render: (v: number) => `${v.toFixed(1)}%` },
              ]}
            />
          </Card>
        </Col>
      </Row>

      <Card title="近 12 个月收支趋势" style={{ marginTop: 16 }}>
        <ReactECharts option={trendOption} style={{ height: 360 }} />
      </Card>
    </div>
  );
}
