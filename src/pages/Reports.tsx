import { useCallback, useEffect, useMemo, useState } from "react";
import {
  Button,
  Card,
  Col,
  DatePicker,
  Form,
  InputNumber,
  Popconfirm,
  Progress,
  Row,
  Select,
  Spin,
  Table,
  Tabs,
  message,
} from "antd";
import ReactECharts from "echarts-for-react";
import dayjs from "dayjs";
import { api, formatMoney } from "../api";
import type { Budget, Category, CategoryStat, MonthlyStat, PortfolioHistoryPoint } from "../types";

export default function ReportsPage() {
  const [loading, setLoading] = useState(true);
  const [month, setMonth] = useState(dayjs().format("YYYY-MM"));
  const [expenseStats, setExpenseStats] = useState<CategoryStat[]>([]);
  const [incomeStats, setIncomeStats] = useState<CategoryStat[]>([]);
  const [monthlyStats, setMonthlyStats] = useState<MonthlyStat[]>([]);
  const [budgets, setBudgets] = useState<Budget[]>([]);
  const [portfolioHistory, setPortfolioHistory] = useState<PortfolioHistoryPoint[]>([]);
  const [categories, setCategories] = useState<Category[]>([]);
  const [budgetForm] = Form.useForm();

  const loadData = useCallback(async () => {
    setLoading(true);
    try {
      const [expense, income, monthly, budgetList, portfolio, expenseCats] = await Promise.all([
        api.getCategoryStats(month, "expense"),
        api.getCategoryStats(month, "income"),
        api.getMonthlyStats(12),
        api.listBudgets(month),
        api.getPortfolioHistory(60),
        api.listCategories("expense"),
      ]);
      setExpenseStats(expense);
      setIncomeStats(income);
      setMonthlyStats(monthly);
      setBudgets(budgetList);
      setPortfolioHistory(portfolio);
      setCategories(expenseCats);
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

  const portfolioOption = useMemo(() => ({
    tooltip: { trigger: "axis" },
    xAxis: { type: "category", data: portfolioHistory.map((p) => p.date) },
    yAxis: { type: "value" },
    series: [{
      name: "持仓市值",
      type: "line",
      data: portfolioHistory.map((p) => p.total_value),
      smooth: true,
      areaStyle: { color: "rgba(22, 119, 255, 0.12)" },
      lineStyle: { color: "#1677ff" },
    }],
  }), [portfolioHistory]);

  const handleSetBudget = async () => {
    const values = await budgetForm.validateFields();
    await api.setBudget({ category_id: values.category_id, month, amount: values.amount });
    message.success("预算已设置");
    budgetForm.resetFields();
    loadData();
  };

  if (loading) {
    return <div style={{ textAlign: "center", padding: 80 }}><Spin size="large" /></div>;
  }

  const totalExpense = expenseStats.reduce((s, i) => s + i.amount, 0);
  const totalIncome = incomeStats.reduce((s, i) => s + i.amount, 0);

  return (
    <div>
      <div className="page-header">
        <h2>报表分析</h2>
        <p>分类统计、预算管控与投资市值变化</p>
      </div>

      <Card style={{ marginBottom: 16 }}>
        <DatePicker picker="month" value={dayjs(month)} onChange={(v) => setMonth(v?.format("YYYY-MM") ?? month)} />
        <span style={{ marginLeft: 24 }}>
          本月收入 {formatMoney(totalIncome)} · 支出 {formatMoney(totalExpense)} · 结余 {formatMoney(totalIncome - totalExpense)}
        </span>
      </Card>

      <Tabs
        items={[
          {
            key: "stats",
            label: "收支分析",
            children: (
              <>
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
                        <div style={{ textAlign: "center", padding: 40, color: "#999" }}>本月暂无支出</div>
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
                        locale={{ emptyText: "本月暂无收入" }}
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
              </>
            ),
          },
          {
            key: "budget",
            label: "预算管理",
            children: (
              <Card>
                <Form form={budgetForm} layout="inline" style={{ marginBottom: 16 }}>
                  <Form.Item name="category_id" rules={[{ required: true, message: "选择分类" }]}>
                    <Select
                      placeholder="支出分类"
                      style={{ width: 160 }}
                      options={categories.map((c) => ({ label: `${c.icon} ${c.name}`, value: c.id }))}
                    />
                  </Form.Item>
                  <Form.Item name="amount" rules={[{ required: true, message: "输入预算" }]}>
                    <InputNumber min={1} prefix="¥" placeholder="月预算" />
                  </Form.Item>
                  <Button type="primary" onClick={handleSetBudget}>设置预算</Button>
                </Form>
                <Table<Budget>
                  rowKey="id"
                  dataSource={budgets}
                  locale={{ emptyText: "本月尚未设置预算" }}
                  columns={[
                    { title: "分类", render: (_, r) => `${r.category_icon} ${r.category_name}` },
                    { title: "预算", dataIndex: "amount", render: (v: number) => formatMoney(v) },
                    { title: "已用", dataIndex: "spent", render: (v: number) => formatMoney(v) },
                    {
                      title: "进度",
                      render: (_, r) => (
                        <Progress
                          percent={Math.min(r.usage_rate, 100)}
                          status={r.is_over ? "exception" : "normal"}
                          format={() => `${r.usage_rate.toFixed(0)}%`}
                        />
                      ),
                    },
                    {
                      title: "操作",
                      render: (_, r) => (
                        <Popconfirm title="删除该预算？" onConfirm={async () => { await api.deleteBudget(r.id); loadData(); }}>
                          <Button type="link" danger>删除</Button>
                        </Popconfirm>
                      ),
                    },
                  ]}
                />
              </Card>
            ),
          },
          {
            key: "portfolio",
            label: "投资市值",
            children: (
              <Card title="持仓市值变化（基于本地行情记录）">
                {portfolioHistory.length > 0 ? (
                  <ReactECharts option={portfolioOption} style={{ height: 400 }} />
                ) : (
                  <div style={{ textAlign: "center", padding: 60, color: "#999" }}>
                    暂无历史数据，添加持仓并刷新几次行情后即可看到趋势
                  </div>
                )}
              </Card>
            ),
          },
        ]}
      />
    </div>
  );
}
