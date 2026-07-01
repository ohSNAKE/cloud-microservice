import { useCallback, useEffect, useState } from "react";
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
  Tag,
  message,
} from "antd";
import { DeleteOutlined, PlusOutlined } from "@ant-design/icons";
import dayjs from "dayjs";
import { api, formatMoney } from "../api";
import type { Account, Category, Transaction } from "../types";

export default function TransactionsPage() {
  const [loading, setLoading] = useState(false);
  const [transactions, setTransactions] = useState<Transaction[]>([]);
  const [categories, setCategories] = useState<Category[]>([]);
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [month, setMonth] = useState(dayjs().format("YYYY-MM"));
  const [open, setOpen] = useState(false);
  const [form] = Form.useForm();

  const loadData = useCallback(async () => {
    setLoading(true);
    try {
      const [txList, incomeCats, expenseCats, accountList] = await Promise.all([
        api.listTransactions(month),
        api.listCategories("income"),
        api.listCategories("expense"),
        api.listAccounts(),
      ]);
      setTransactions(txList);
      setCategories([...incomeCats, ...expenseCats]);
      setAccounts(accountList);
    } finally {
      setLoading(false);
    }
  }, [month]);

  useEffect(() => {
    loadData();
  }, [loadData]);

  const handleSubmit = async () => {
    const values = await form.validateFields();
    await api.addTransaction({
      type: values.type,
      amount: values.amount,
      category_id: values.category_id,
      account_id: values.account_id,
      note: values.note,
      transaction_date: values.transaction_date.format("YYYY-MM-DD"),
    });
    message.success("记账成功");
    setOpen(false);
    form.resetFields();
    loadData();
  };

  const handleDelete = async (id: number) => {
    await api.deleteTransaction(id);
    message.success("已删除");
    loadData();
  };

  const txType = Form.useWatch("type", form);

  return (
    <div>
      <div className="page-header">
        <h2>记账</h2>
        <p>记录日常收入与支出</p>
      </div>

      <Card>
        <Space style={{ marginBottom: 16 }} wrap>
          <DatePicker
            picker="month"
            value={dayjs(month)}
            onChange={(value) => setMonth(value?.format("YYYY-MM") ?? dayjs().format("YYYY-MM"))}
          />
          <Button type="primary" icon={<PlusOutlined />} onClick={() => setOpen(true)}>
            新增记账
          </Button>
        </Space>

        <Table
          rowKey="id"
          loading={loading}
          dataSource={transactions}
          pagination={{ pageSize: 10 }}
          columns={[
            {
              title: "日期",
              dataIndex: "transaction_date",
              width: 120,
            },
            {
              title: "类型",
              dataIndex: "type",
              width: 90,
              render: (type: string) =>
                type === "income" ? <Tag color="green">收入</Tag> : <Tag color="red">支出</Tag>,
            },
            {
              title: "分类",
              render: (_, record) => `${record.category_icon ?? ""} ${record.category_name ?? "-"}`,
            },
            {
              title: "账户",
              dataIndex: "account_name",
              render: (value) => value ?? "-",
            },
            {
              title: "金额",
              dataIndex: "amount",
              render: (value: number, record) => (
                <span className={record.type === "income" ? "profit-negative" : "profit-positive"}>
                  {record.type === "income" ? "+" : "-"}
                  {formatMoney(value)}
                </span>
              ),
            },
            {
              title: "备注",
              dataIndex: "note",
              ellipsis: true,
            },
            {
              title: "操作",
              width: 80,
              render: (_, record) => (
                <Popconfirm title="确认删除这条记录？" onConfirm={() => handleDelete(record.id)}>
                  <Button type="text" danger icon={<DeleteOutlined />} />
                </Popconfirm>
              ),
            },
          ]}
        />
      </Card>

      <Modal
        title="新增记账"
        open={open}
        onCancel={() => setOpen(false)}
        onOk={handleSubmit}
        destroyOnClose
      >
        <Form
          form={form}
          layout="vertical"
          initialValues={{
            type: "expense",
            transaction_date: dayjs(),
          }}
        >
          <Form.Item name="type" label="类型" rules={[{ required: true }]}>
            <Select
              options={[
                { label: "支出", value: "expense" },
                { label: "收入", value: "income" },
              ]}
            />
          </Form.Item>
          <Form.Item name="amount" label="金额" rules={[{ required: true }]}>
            <InputNumber min={0.01} precision={2} style={{ width: "100%" }} prefix="¥" />
          </Form.Item>
          <Form.Item name="category_id" label="分类" rules={[{ required: true }]}>
            <Select
              options={categories
                .filter((item) => item.type === (txType ?? "expense"))
                .map((item) => ({
                  label: `${item.icon} ${item.name}`,
                  value: item.id,
                }))}
            />
          </Form.Item>
          <Form.Item name="account_id" label="账户" rules={[{ required: true }]}>
            <Select options={accounts.map((item) => ({ label: item.name, value: item.id }))} />
          </Form.Item>
          <Form.Item name="transaction_date" label="日期" rules={[{ required: true }]}>
            <DatePicker style={{ width: "100%" }} />
          </Form.Item>
          <Form.Item name="note" label="备注">
            <Input.TextArea rows={2} placeholder="可选" />
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
}
