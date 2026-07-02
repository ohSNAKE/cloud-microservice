import { useCallback, useEffect, useMemo, useState } from "react";
import {
  Button,
  Card,
  DatePicker,
  Form,
  InputNumber,
  Popconfirm,
  Progress,
  Select,
  Table,
  Tabs,
  message,
} from "antd";
import ReactECharts from "echarts-for-react";
import dayjs from "dayjs";
import { api, formatMoney } from "../api";
import EmptyPlaceholder from "../components/layout/EmptyPlaceholder";
import PageHeader from "../components/layout/PageHeader";
import PageLoader from "../components/layout/PageLoader";
import { chartGrid } from "../constants/chartTheme";
import { useTheme } from "../context/ThemeContext";
import type { Budget, Category, CategoryStat, MonthlyStat, PortfolioHistoryPoint } from "../types";

export default function ReportsPage() {
  const { chartColors } = useTheme();
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

  const expenseChartOption = useMemo(
    () => ({
      tooltip: { trigger: "item" },
      color: chartColors.expensePalette,
      series: [
        {
          type: "pie",
          radius: "65%",
          data: expenseStats.map((s) => ({
            name: `${s.category_icon} ${s.category_name}`,
            value: s.amount,
          })),
        },
      ],
    }),
    [expenseStats, chartColors],
  );

  const trendOption = useMemo(
    () => ({
      tooltip: { trigger: "axis" },
      legend: { data: ["收入", "支出"] },
      grid: chartGrid,
      xAxis: { type: "category", data: monthlyStats.map((m) => m.month) },
      yAxis: { type: "value" },
      series: [
        {
          name: "收入",
          type: "line",
          data: monthlyStats.map((m) => m.income),
          smooth: true,
          itemStyle: { color: chartColors.income },
        },
        {
          name: "支出",
          type: "line",
          data: monthlyStats.map((m) => m.expense),
          smooth: true,
          itemStyle: { color: chartColors.expense },
        },
      ],
    }),
    [monthlyStats, chartColors],
  );

  const portfolioOption = useMemo(
    () => ({
      tooltip: { trigger: "axis" },
      grid: chartGrid,
      xAxis: { type: "category", data: portfolioHistory.map((p) => p.date) },
      yAxis: { type: "value" },
      series: [
        {
          name: "持仓市值",
          type: "line",
          data: portfolioHistory.map((p) => p.total_value),
          smooth: true,
          areaStyle: { color: chartColors.primaryArea },
          lineStyle: { color: chartColors.primary },
        },
      ],
    }),
    [portfolioHistory, chartColors],
  );

  const handleSetBudget = async () => {
    const values = await budgetForm.validateFields();
    await api.setBudget({ category_id: values.category_id, month, amount: values.amount });
    message.success("预算已设置");
    budgetForm.resetFields();
    loadData();
  };

  if (loading) {
    return <PageLoader tip="加载报表数据..." />;
  }

  const totalExpense = expenseStats.reduce((s, i) => s + i.amount, 0);
  const totalIncome = incomeStats.reduce((s, i) => s + i.amount, 0);

  return (
    <div>
      <PageHeader
        actions={
          <DatePicker
            picker="month"
            value={dayjs(month)}
            onChange={(v) => setMonth(v?.format("YYYY-MM") ?? month)}
          />
        }
      />

      <Card className="stat-card" style={{ marginBottom: 16 }}>
        <div className="summary-strip">
          <span className="summary-strip__item">
            本月收入：<strong className="amount-income">{formatMoney(totalIncome)}</strong>
          </span>
          <span className="summary-strip__item">
            本月支出：<strong className="amount-expense">{formatMoney(totalExpense)}</strong>
          </span>
          <span className="summary-strip__item">
            结余：<strong>{formatMoney(totalIncome - totalExpense)}</strong>
          </span>
        </div>
      </Card>

      <Tabs
        items={[
          {
            key: "stats",
            label: "收支分析",
            children: (
              <>
                <div className="content-grid content-grid--2">
                  <Card className="stat-card content-grid__item" title="支出分类占比">
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
                        <EmptyPlaceholder description="本月暂无支出记录" />
                      )}
                    </Card>
                  <Card className="stat-card content-grid__item" title="收入来源">
                      {incomeStats.length > 0 ? (
                        <Table<CategoryStat>
                          size="small"
                          rowKey="category_id"
                          pagination={false}
                          dataSource={incomeStats}
                          columns={[
                            { title: "分类", render: (_, r) => `${r.category_icon} ${r.category_name}` },
                            { title: "金额", dataIndex: "amount", render: (v: number) => formatMoney(v) },
                            { title: "占比", dataIndex: "percentage", render: (v: number) => `${v.toFixed(1)}%` },
                          ]}
                        />
                      ) : (
                        <EmptyPlaceholder description="本月暂无收入记录" />
                      )}
                    </Card>
                </div>
                <Card className="stat-card" title="近 12 个月收支趋势" style={{ marginTop: 16 }}>
                  <ReactECharts option={trendOption} style={{ height: 360 }} />
                </Card>
              </>
            ),
          },
          {
            key: "budget",
            label: "预算管理",
            children: (
              <Card className="stat-card">
                <Form form={budgetForm} layout="inline" className="filter-bar" style={{ marginBottom: 0 }}>
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
                  <Button type="primary" onClick={handleSetBudget}>
                    设置预算
                  </Button>
                </Form>
                <Table<Budget>
                  rowKey="id"
                  dataSource={budgets}
                  style={{ marginTop: 16 }}
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
                        <Popconfirm
                          title="删除该预算？"
                          onConfirm={async () => {
                            await api.deleteBudget(r.id);
                            loadData();
                          }}
                        >
                          <Button type="link" danger>
                            删除
                          </Button>
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
              <Card className="stat-card" title="持仓市值变化（基于本地行情记录）">
                {portfolioHistory.length > 0 ? (
                  <ReactECharts option={portfolioOption} style={{ height: 400 }} />
                ) : (
                  <EmptyPlaceholder description="暂无历史数据，添加持仓并刷新几次行情后即可看到趋势" />
                )}
              </Card>
            ),
          },
        ]}
      />
    </div>
  );
}
