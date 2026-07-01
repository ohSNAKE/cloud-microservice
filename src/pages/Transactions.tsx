import { useCallback, useEffect, useMemo, useState } from "react";
import { useSearchParams } from "react-router-dom";
import {
  Button,
  Card,
  DatePicker,
  Form,
  Input,
  InputNumber,
  Modal,
  Popconfirm,
  Select,
  Space,
  Table,
  Tabs,
  Tag,
  message,
} from "antd";
import {
  DeleteOutlined,
  EditOutlined,
  PlusOutlined,
  SwapOutlined,
} from "@ant-design/icons";
import dayjs from "dayjs";
import { api, accountTypeLabel, formatMoney } from "../api";
import PageHeader from "../components/layout/PageHeader";
import { useQuickAdd } from "../context/QuickAddContext";
import type {
  Account,
  Category,
  NewAccount,
  NewCategory,
  NewRecurringRule,
  RecurringRule,
  Transaction,
  UpdateAccount,
  UpdateCategory,
  UpdateTransaction,
} from "../types";
import { ACCOUNT_TYPES } from "../types";

type TxFormMode = "expense" | "income" | "transfer" | "edit";

export default function TransactionsPage() {
  const [searchParams] = useSearchParams();
  const { openQuickAdd } = useQuickAdd();
  const [loading, setLoading] = useState(false);
  const [transactions, setTransactions] = useState<Transaction[]>([]);
  const [categories, setCategories] = useState<Category[]>([]);
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [month, setMonth] = useState(dayjs().format("YYYY-MM"));
  const [accountFilter, setAccountFilter] = useState<number | undefined>();
  const [categoryFilter, setCategoryFilter] = useState<number | undefined>();
  const [typeFilter, setTypeFilter] = useState<string | undefined>();
  const [keyword, setKeyword] = useState("");
  const [modalOpen, setModalOpen] = useState(false);
  const [modalMode, setModalMode] = useState<TxFormMode>("expense");
  const [editingId, setEditingId] = useState<number | null>(null);
  const [form] = Form.useForm();

  const [accountModal, setAccountModal] = useState(false);
  const [categoryModal, setCategoryModal] = useState(false);
  const [accountForm] = Form.useForm();
  const [categoryForm] = Form.useForm();
  const [editingAccount, setEditingAccount] = useState<Account | null>(null);
  const [editingCategory, setEditingCategory] = useState<Category | null>(null);
  const [recurringRules, setRecurringRules] = useState<RecurringRule[]>([]);
  const [recurringModal, setRecurringModal] = useState(false);
  const [recurringForm] = Form.useForm();

  const loadData = useCallback(async () => {
    setLoading(true);
    try {
      const [txList, incomeCats, expenseCats, accountList, recurring] = await Promise.all([
        api.listTransactions({
          month,
          account_id: accountFilter,
          category_id: categoryFilter,
          tx_type: typeFilter,
          keyword: keyword || undefined,
        }),
        api.listCategories("income"),
        api.listCategories("expense"),
        api.listAccounts(),
        api.listRecurringRules(),
      ]);
      setTransactions(txList);
      setCategories([...incomeCats, ...expenseCats]);
      setAccounts(accountList);
      setRecurringRules(recurring);
    } finally {
      setLoading(false);
    }
  }, [month, accountFilter, categoryFilter, typeFilter, keyword]);

  useEffect(() => {
    loadData();
  }, [loadData]);

  useEffect(() => {
    const action = searchParams.get("action");
    if (action === "expense" || action === "income") {
      openModal(action);
    }
  }, [searchParams]);

  const monthSummary = useMemo(() => {
    const income = transactions
      .filter((t) => t.type === "income")
      .reduce((s, t) => s + t.amount, 0);
    const expense = transactions
      .filter((t) => t.type === "expense")
      .reduce((s, t) => s + t.amount, 0);
    return { income, expense, balance: income - expense };
  }, [transactions]);

  const openModal = (mode: TxFormMode, record?: Transaction) => {
    setModalMode(mode);
    setEditingId(record?.id ?? null);
    form.resetFields();
    if (mode === "edit" && record) {
      form.setFieldsValue({
        type: record.type,
        amount: record.amount,
        category_id: record.category_id,
        account_id: record.account_id,
        note: record.note,
        transaction_date: dayjs(record.transaction_date),
      });
      setModalMode("edit");
    } else if (mode === "transfer") {
      form.setFieldsValue({ transaction_date: dayjs() });
    } else {
      form.setFieldsValue({ type: mode, transaction_date: dayjs() });
    }
    setModalOpen(true);
  };

  const handleSubmit = async () => {
    const values = await form.validateFields();
    const date = values.transaction_date.format("YYYY-MM-DD");

    if (modalMode === "transfer") {
      await api.addTransfer({
        from_account_id: values.from_account_id,
        to_account_id: values.to_account_id,
        amount: values.amount,
        note: values.note,
        transaction_date: date,
      });
      message.success("转账成功");
    } else if (modalMode === "edit" && editingId) {
      const input: UpdateTransaction = {
        type: values.type,
        amount: values.amount,
        category_id: values.category_id,
        account_id: values.account_id,
        note: values.note,
        transaction_date: date,
      };
      await api.updateTransaction(editingId, input);
      message.success("已更新");
    } else {
      await api.addTransaction({
        type: values.type,
        amount: values.amount,
        category_id: values.category_id,
        account_id: values.account_id,
        note: values.note,
        transaction_date: date,
      });
      message.success("记账成功");
    }
    setModalOpen(false);
    loadData();
  };

  const txType = Form.useWatch("type", form);

  const saveAccount = async () => {
    const values = await accountForm.validateFields();
    if (editingAccount) {
      await api.updateAccount(editingAccount.id, values as UpdateAccount);
      message.success("账户已更新");
    } else {
      await api.addAccount(values as NewAccount);
      message.success("账户已添加");
    }
    setAccountModal(false);
    loadData();
  };

  const saveCategory = async () => {
    const values = await categoryForm.validateFields();
    if (editingCategory) {
      await api.updateCategory(editingCategory.id, values as UpdateCategory);
      message.success("分类已更新");
    } else {
      await api.addCategory(values as NewCategory);
      message.success("分类已添加");
    }
    setCategoryModal(false);
    loadData();
  };

  const saveRecurring = async () => {
    const values = await recurringForm.validateFields();
    await api.addRecurringRule(values as NewRecurringRule);
    message.success("周期记账已添加，将在每月指定日期自动记账");
    setRecurringModal(false);
    recurringForm.resetFields();
    loadData();
  };

  return (
    <div>
      <PageHeader
        actions={
          <>
            <Button type="primary" icon={<PlusOutlined />} onClick={openQuickAdd}>
              智能记账
            </Button>
            <Button onClick={() => openModal("expense")}>记支出</Button>
            <Button onClick={() => openModal("income")}>记收入</Button>
            <Button icon={<SwapOutlined />} onClick={() => openModal("transfer")}>
              转账
            </Button>
          </>
        }
      />

      <Card className="stat-card" style={{ marginBottom: 16 }}>
        <div className="summary-strip">
          <span className="summary-strip__item">
            本月收入：<strong className="amount-income">{formatMoney(monthSummary.income)}</strong>
          </span>
          <span className="summary-strip__item">
            本月支出：<strong className="amount-expense">{formatMoney(monthSummary.expense)}</strong>
          </span>
          <span className="summary-strip__item">
            结余：<strong>{formatMoney(monthSummary.balance)}</strong>
          </span>
        </div>
      </Card>

      <Card className="stat-card">
        <div className="filter-bar">
          <DatePicker picker="month" value={dayjs(month)} onChange={(v) => setMonth(v?.format("YYYY-MM") ?? month)} />
          <Select
            allowClear
            placeholder="账户"
            style={{ width: 120 }}
            value={accountFilter}
            onChange={setAccountFilter}
            options={accounts.map((a) => ({ label: a.name, value: a.id }))}
          />
          <Select
            allowClear
            placeholder="类型"
            style={{ width: 100 }}
            value={typeFilter}
            onChange={setTypeFilter}
            options={[
              { label: "收入", value: "income" },
              { label: "支出", value: "expense" },
              { label: "转账", value: "transfer" },
            ]}
          />
          <Select
            allowClear
            placeholder="分类"
            style={{ width: 120 }}
            value={categoryFilter}
            onChange={setCategoryFilter}
            options={categories.map((c) => ({ label: `${c.icon} ${c.name}`, value: c.id }))}
          />
          <Input.Search placeholder="搜索备注" allowClear onSearch={setKeyword} style={{ width: 160 }} />
        </div>

        <Tabs
          items={[
            {
              key: "list",
              label: "流水",
              children: (
                <Table
                  rowKey="id"
                  loading={loading}
                  dataSource={transactions}
                  pagination={{ pageSize: 10 }}
                  columns={[
                    { title: "日期", dataIndex: "transaction_date", width: 110 },
                    {
                      title: "类型",
                      dataIndex: "type",
                      width: 90,
                      render: (type: string) => {
                        if (type === "income") return <Tag color="green">收入</Tag>;
                        if (type === "transfer") return <Tag color="blue">转账</Tag>;
                        return <Tag color="red">支出</Tag>;
                      },
                    },
                    {
                      title: "分类/说明",
                      render: (_, r) =>
                        r.type === "transfer"
                          ? `${r.account_name} → ${r.transfer_to_account_name}`
                          : `${r.category_icon ?? ""} ${r.category_name ?? "-"}`,
                    },
                    { title: "账户", dataIndex: "account_name", render: (v) => v ?? "-" },
                    {
                      title: "金额",
                      render: (_, r) => {
                        if (r.type === "transfer") return formatMoney(r.amount);
                        return (
                          <span className={r.type === "income" ? "profit-negative" : "profit-positive"}>
                            {r.type === "income" ? "+" : "-"}
                            {formatMoney(r.amount)}
                          </span>
                        );
                      },
                    },
                    { title: "备注", dataIndex: "note", ellipsis: true },
                    {
                      title: "操作",
                      width: 100,
                      render: (_, r) =>
                        r.type === "transfer" ? (
                          <Popconfirm title="确认删除？" onConfirm={async () => { await api.deleteTransaction(r.id); loadData(); }}>
                            <Button type="text" danger icon={<DeleteOutlined />} />
                          </Popconfirm>
                        ) : (
                          <Space>
                            <Button type="text" icon={<EditOutlined />} onClick={() => openModal("edit", r)} />
                            <Popconfirm title="确认删除？" onConfirm={async () => { await api.deleteTransaction(r.id); message.success("已删除"); loadData(); }}>
                              <Button type="text" danger icon={<DeleteOutlined />} />
                            </Popconfirm>
                          </Space>
                        ),
                    },
                  ]}
                />
              ),
            },
            {
              key: "accounts",
              label: "账户管理",
              children: (
                <>
                  <Button type="primary" style={{ marginBottom: 12 }} onClick={() => { setEditingAccount(null); accountForm.resetFields(); setAccountModal(true); }}>
                    添加账户
                  </Button>
                  <Table
                    rowKey="id"
                    dataSource={accounts}
                    pagination={false}
                    columns={[
                      { title: "名称", dataIndex: "name" },
                      { title: "类型", dataIndex: "type", render: accountTypeLabel },
                      { title: "余额", dataIndex: "balance", render: (v: number) => formatMoney(v) },
                      {
                        title: "操作",
                        render: (_, r) => (
                          <Space>
                            <Button type="link" onClick={() => { setEditingAccount(r); accountForm.setFieldsValue(r); setAccountModal(true); }}>编辑</Button>
                            <Popconfirm title="确认删除？" onConfirm={async () => { try { await api.deleteAccount(r.id); message.success("已删除"); loadData(); } catch (e) { message.error(String(e)); } }}>
                              <Button type="link" danger>删除</Button>
                            </Popconfirm>
                          </Space>
                        ),
                      },
                    ]}
                  />
                </>
              ),
            },
            {
              key: "categories",
              label: "分类管理",
              children: (
                <>
                  <Button type="primary" style={{ marginBottom: 12 }} onClick={() => { setEditingCategory(null); categoryForm.resetFields(); categoryForm.setFieldsValue({ type: "expense", icon: "📌" }); setCategoryModal(true); }}>
                    添加分类
                  </Button>
                  <Table
                    rowKey="id"
                    dataSource={categories}
                    pagination={false}
                    columns={[
                      { title: "图标", dataIndex: "icon", width: 60 },
                      { title: "名称", dataIndex: "name" },
                      { title: "类型", dataIndex: "type", render: (t: string) => (t === "income" ? "收入" : "支出") },
                      {
                        title: "操作",
                        render: (_, r) => (
                          <Space>
                            <Button type="link" onClick={() => { setEditingCategory(r); categoryForm.setFieldsValue(r); setCategoryModal(true); }}>编辑</Button>
                            <Popconfirm title="确认删除？" onConfirm={async () => { try { await api.deleteCategory(r.id); message.success("已删除"); loadData(); } catch (e) { message.error(String(e)); } }}>
                              <Button type="link" danger>删除</Button>
                            </Popconfirm>
                          </Space>
                        ),
                      },
                    ]}
                  />
                </>
              ),
            },
            {
              key: "recurring",
              label: "周期记账",
              children: (
                <>
                  <Button type="primary" style={{ marginBottom: 12 }} onClick={() => { recurringForm.resetFields(); recurringForm.setFieldsValue({ type: "expense", day_of_month: 1 }); setRecurringModal(true); }}>
                    添加周期规则
                  </Button>
                  <Table<RecurringRule>
                    rowKey="id"
                    dataSource={recurringRules}
                    pagination={false}
                    locale={{ emptyText: "如工资、房租等固定收支，可在此设置自动记账" }}
                    columns={[
                      { title: "类型", dataIndex: "type", render: (t: string) => (t === "income" ? "收入" : "支出") },
                      { title: "金额", dataIndex: "amount", render: (v: number) => formatMoney(v) },
                      { title: "分类", render: (_, r) => `${r.category_icon ?? ""} ${r.category_name ?? "-"}` },
                      { title: "账户", dataIndex: "account_name", render: (v) => v ?? "-" },
                      { title: "每月", dataIndex: "day_of_month", render: (d: number) => `${d} 日` },
                      { title: "上次执行", dataIndex: "last_run_month", render: (v) => v ?? "未执行" },
                      {
                        title: "启用",
                        dataIndex: "enabled",
                        render: (enabled: boolean, r) => (
                          <Button type="link" onClick={async () => { await api.toggleRecurringRule(r.id, !enabled); loadData(); }}>
                            {enabled ? "已启用" : "已停用"}
                          </Button>
                        ),
                      },
                      {
                        title: "操作",
                        render: (_, r) => (
                          <Popconfirm title="确认删除？" onConfirm={async () => { await api.deleteRecurringRule(r.id); message.success("已删除"); loadData(); }}>
                            <Button type="link" danger>删除</Button>
                          </Popconfirm>
                        ),
                      },
                    ]}
                  />
                </>
              ),
            },
          ]}
        />
      </Card>

      <Modal
        title={
          modalMode === "transfer" ? "账户转账" :
          modalMode === "edit" ? "编辑记录" :
          modalMode === "income" ? "记收入" : "记支出"
        }
        open={modalOpen}
        onCancel={() => setModalOpen(false)}
        onOk={handleSubmit}
        destroyOnClose
      >
        <Form form={form} layout="vertical">
          {modalMode === "transfer" ? (
            <>
              <Form.Item name="from_account_id" label="转出账户" rules={[{ required: true }]}>
                <Select options={accounts.map((a) => ({ label: a.name, value: a.id }))} />
              </Form.Item>
              <Form.Item name="to_account_id" label="转入账户" rules={[{ required: true }]}>
                <Select options={accounts.map((a) => ({ label: a.name, value: a.id }))} />
              </Form.Item>
              <Form.Item name="amount" label="金额" rules={[{ required: true }]}>
                <InputNumber min={0.01} precision={2} style={{ width: "100%" }} prefix="¥" />
              </Form.Item>
              <Form.Item name="transaction_date" label="日期" rules={[{ required: true }]}>
                <DatePicker style={{ width: "100%" }} />
              </Form.Item>
              <Form.Item name="note" label="备注"><Input placeholder="可选" /></Form.Item>
            </>
          ) : (
            <>
              {modalMode !== "edit" && (
                <Form.Item name="type" label="类型" rules={[{ required: true }]}>
                  <Select options={[{ label: "支出", value: "expense" }, { label: "收入", value: "income" }]} />
                </Form.Item>
              )}
              {modalMode === "edit" && <Form.Item name="type" hidden><Input /></Form.Item>}
              <Form.Item name="amount" label="金额" rules={[{ required: true }]}>
                <InputNumber min={0.01} precision={2} style={{ width: "100%" }} prefix="¥" />
              </Form.Item>
              <Form.Item name="category_id" label="分类" rules={[{ required: true }]}>
                <Select options={categories.filter((c) => c.type === (txType ?? "expense")).map((c) => ({ label: `${c.icon} ${c.name}`, value: c.id }))} />
              </Form.Item>
              <Form.Item name="account_id" label="账户" rules={[{ required: true }]}>
                <Select options={accounts.map((a) => ({ label: a.name, value: a.id }))} />
              </Form.Item>
              <Form.Item name="transaction_date" label="日期" rules={[{ required: true }]}>
                <DatePicker style={{ width: "100%" }} />
              </Form.Item>
              <Form.Item name="note" label="备注"><Input.TextArea rows={2} /></Form.Item>
            </>
          )}
        </Form>
      </Modal>

      <Modal title={editingAccount ? "编辑账户" : "添加账户"} open={accountModal} onCancel={() => setAccountModal(false)} onOk={saveAccount} destroyOnClose>
        <Form form={accountForm} layout="vertical">
          <Form.Item name="name" label="名称" rules={[{ required: true }]}><Input /></Form.Item>
          <Form.Item name="type" label="类型" rules={[{ required: true }]}>
            <Select options={ACCOUNT_TYPES} />
          </Form.Item>
          <Form.Item name="balance" label="当前余额" rules={[{ required: true }]}>
            <InputNumber min={0} precision={2} style={{ width: "100%" }} prefix="¥" />
          </Form.Item>
        </Form>
      </Modal>

      <Modal title={editingCategory ? "编辑分类" : "添加分类"} open={categoryModal} onCancel={() => setCategoryModal(false)} onOk={saveCategory} destroyOnClose>
        <Form form={categoryForm} layout="vertical">
          <Form.Item name="name" label="名称" rules={[{ required: true }]}><Input /></Form.Item>
          <Form.Item name="type" label="类型" rules={[{ required: true }]}>
            <Select options={[{ label: "支出", value: "expense" }, { label: "收入", value: "income" }]} disabled={!!editingCategory} />
          </Form.Item>
          <Form.Item name="icon" label="图标"><Input placeholder="如 🍜" /></Form.Item>
        </Form>
      </Modal>

      <Modal title="添加周期记账" open={recurringModal} onCancel={() => setRecurringModal(false)} onOk={saveRecurring} destroyOnClose>
        <Form form={recurringForm} layout="vertical">
          <Form.Item name="type" label="类型" rules={[{ required: true }]}>
            <Select options={[{ label: "支出", value: "expense" }, { label: "收入", value: "income" }]} />
          </Form.Item>
          <Form.Item name="amount" label="金额" rules={[{ required: true }]}>
            <InputNumber min={0.01} precision={2} style={{ width: "100%" }} prefix="¥" />
          </Form.Item>
          <Form.Item name="category_id" label="分类" rules={[{ required: true }]}>
            <Select options={categories.map((c) => ({ label: `${c.icon} ${c.name}`, value: c.id }))} />
          </Form.Item>
          <Form.Item name="account_id" label="账户" rules={[{ required: true }]}>
            <Select options={accounts.map((a) => ({ label: a.name, value: a.id }))} />
          </Form.Item>
          <Form.Item name="day_of_month" label="每月几号记账" rules={[{ required: true }]}>
            <InputNumber min={1} max={28} style={{ width: "100%" }} />
          </Form.Item>
          <Form.Item name="note" label="备注"><Input placeholder="如：房租、工资" /></Form.Item>
        </Form>
      </Modal>
    </div>
  );
}
