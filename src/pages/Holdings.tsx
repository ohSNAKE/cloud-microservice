import { useCallback, useEffect, useState } from "react";
import {
  Button,
  Card,
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
import { DeleteOutlined, PlusOutlined, SyncOutlined } from "@ant-design/icons";
import { api, formatMoney, formatPercent } from "../api";
import type { Holding } from "../types";

export default function HoldingsPage() {
  const [loading, setLoading] = useState(false);
  const [refreshing, setRefreshing] = useState(false);
  const [holdings, setHoldings] = useState<Holding[]>([]);
  const [open, setOpen] = useState(false);
  const [form] = Form.useForm();

  const loadData = useCallback(async () => {
    setLoading(true);
    try {
      setHoldings(await api.listHoldings());
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadData();
  }, [loadData]);

  const handleRefresh = async () => {
    setRefreshing(true);
    try {
      const result = await api.refreshQuotes();
      message.success(result.message);
      loadData();
    } catch (error) {
      message.error(String(error));
    } finally {
      setRefreshing(false);
    }
  };

  const handleSubmit = async () => {
    const values = await form.validateFields();
    await api.addHolding({
      code: values.code,
      name: values.name ?? "",
      type: values.type,
      quantity: values.quantity,
      cost_price: values.cost_price,
    });
    message.success("持仓已添加");
    setOpen(false);
    form.resetFields();
    loadData();
  };

  const handleDelete = async (id: number) => {
    await api.deleteHolding(id);
    message.success("已删除");
    loadData();
  };

  const totalMarketValue = holdings.reduce((sum, item) => sum + item.market_value, 0);
  const totalProfit = holdings.reduce((sum, item) => sum + item.profit, 0);

  return (
    <div>
      <div className="page-header">
        <h2>股票基金</h2>
        <p>管理持仓并查看实时盈亏</p>
      </div>

      <Card style={{ marginBottom: 16 }}>
        <Space size="large" wrap>
          <span>总市值：<strong>{formatMoney(totalMarketValue)}</strong></span>
          <span>
            总盈亏：
            <strong className={totalProfit >= 0 ? "profit-positive" : "profit-negative"}>
              {formatMoney(totalProfit)}
            </strong>
          </span>
        </Space>
      </Card>

      <Card>
        <Space style={{ marginBottom: 16 }} wrap>
          <Button type="primary" icon={<PlusOutlined />} onClick={() => setOpen(true)}>
            添加持仓
          </Button>
          <Button icon={<SyncOutlined />} loading={refreshing} onClick={handleRefresh}>
            刷新行情
          </Button>
        </Space>

        <Table
          rowKey="id"
          loading={loading}
          dataSource={holdings}
          pagination={{ pageSize: 10 }}
          columns={[
            {
              title: "代码",
              dataIndex: "code",
              width: 100,
            },
            {
              title: "名称",
              dataIndex: "name",
            },
            {
              title: "类型",
              dataIndex: "type",
              width: 90,
              render: (type: string) =>
                type === "fund" ? <Tag color="blue">基金</Tag> : <Tag color="purple">股票</Tag>,
            },
            {
              title: "数量",
              dataIndex: "quantity",
            },
            {
              title: "成本价",
              dataIndex: "cost_price",
              render: (value: number) => value.toFixed(3),
            },
            {
              title: "现价",
              dataIndex: "current_price",
              render: (value: number) => value.toFixed(3),
            },
            {
              title: "市值",
              dataIndex: "market_value",
              render: (value: number) => formatMoney(value),
            },
            {
              title: "盈亏",
              render: (_, record) => (
                <span className={record.profit >= 0 ? "profit-positive" : "profit-negative"}>
                  {formatMoney(record.profit)} ({formatPercent(record.profit_rate)})
                </span>
              ),
            },
            {
              title: "更新时间",
              dataIndex: "updated_at",
              width: 170,
            },
            {
              title: "操作",
              width: 80,
              render: (_, record) => (
                <Popconfirm title="确认删除该持仓？" onConfirm={() => handleDelete(record.id)}>
                  <Button type="text" danger icon={<DeleteOutlined />} />
                </Popconfirm>
              ),
            },
          ]}
        />
      </Card>

      <Modal
        title="添加持仓"
        open={open}
        onCancel={() => setOpen(false)}
        onOk={handleSubmit}
        destroyOnClose
      >
        <Form
          form={form}
          layout="vertical"
          initialValues={{ type: "stock", quantity: 100, cost_price: 10 }}
        >
          <Form.Item name="type" label="类型" rules={[{ required: true }]}>
            <Select
              options={[
                { label: "股票", value: "stock" },
                { label: "基金", value: "fund" },
              ]}
            />
          </Form.Item>
          <Form.Item
            name="code"
            label="代码"
            rules={[{ required: true, message: "请输入代码" }]}
            extra="股票如 600519，基金如 000001"
          >
            <Input placeholder="证券代码" />
          </Form.Item>
          <Form.Item name="name" label="名称">
            <Input placeholder="可选，股票会自动尝试获取名称" />
          </Form.Item>
          <Form.Item name="quantity" label="持有数量" rules={[{ required: true }]}>
            <InputNumber min={0.0001} precision={4} style={{ width: "100%" }} />
          </Form.Item>
          <Form.Item name="cost_price" label="成本价" rules={[{ required: true }]}>
            <InputNumber min={0.0001} precision={4} style={{ width: "100%" }} prefix="¥" />
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
}
