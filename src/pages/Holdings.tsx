import { useCallback, useEffect, useMemo, useState } from "react";
import {
  Alert,
  Button,
  Card,
  Col,
  Form,
  Input,
  InputNumber,
  Modal,
  Popconfirm,
  Row,
  Select,
  Space,
  Table,
  Tag,
  message,
} from "antd";
import {
  DeleteOutlined,
  EditOutlined,
  LineChartOutlined,
  PlusOutlined,
  SyncOutlined,
} from "@ant-design/icons";
import ReactECharts from "echarts-for-react";
import { api, formatMoney, formatPercent } from "../api";
import KlineChart from "../components/KlineChart";
import type { Holding } from "../types";

export default function HoldingsPage() {
  const [loading, setLoading] = useState(false);
  const [refreshing, setRefreshing] = useState(false);
  const [holdings, setHoldings] = useState<Holding[]>([]);
  const [failedCodes, setFailedCodes] = useState<string[]>([]);
  const [addOpen, setAddOpen] = useState(false);
  const [editOpen, setEditOpen] = useState(false);
  const [klineHolding, setKlineHolding] = useState<Holding | null>(null);
  const [editing, setEditing] = useState<Holding | null>(null);
  const [addForm] = Form.useForm();
  const [editForm] = Form.useForm();

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
      setFailedCodes(result.failed_codes);
      if (result.failed > 0) {
        message.warning(result.message);
      } else {
        message.success(result.message);
      }
      loadData();
    } catch (error) {
      message.error(String(error));
    } finally {
      setRefreshing(false);
    }
  };

  const handleAdd = async () => {
    const values = await addForm.validateFields();
    await api.addHolding({
      code: values.code,
      name: values.name ?? "",
      type: values.type,
      quantity: values.quantity,
      cost_price: values.cost_price,
    });
    message.success("持仓已添加");
    setAddOpen(false);
    addForm.resetFields();
    loadData();
  };

  const handleEdit = async () => {
    if (!editing) return;
    const values = await editForm.validateFields();
    await api.updateHolding(editing.id, values.quantity, values.cost_price);
    message.success("持仓已更新");
    setEditOpen(false);
    loadData();
  };

  const totalMarketValue = holdings.reduce((sum, item) => sum + item.market_value, 0);
  const totalProfit = holdings.reduce((sum, item) => sum + item.profit, 0);

  const allocationOption = useMemo(() => ({
    tooltip: { trigger: "item" },
    series: [{
      type: "pie",
      radius: ["40%", "65%"],
      data: holdings.map((h) => ({ name: h.name, value: h.market_value })),
    }],
  }), [holdings]);

  return (
    <div>
      <div className="page-header">
        <h2>投资持仓</h2>
        <p>管理股票与基金，追踪市值与盈亏</p>
      </div>

      <Row gutter={[16, 16]} style={{ marginBottom: 16 }}>
        <Col xs={24} md={16}>
          <Card>
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
        </Col>
        <Col xs={24} md={8}>
          {holdings.length > 0 && (
            <Card title="组合占比" bodyStyle={{ padding: 8 }}>
              <ReactECharts option={allocationOption} style={{ height: 120 }} />
            </Card>
          )}
        </Col>
      </Row>

      {failedCodes.length > 0 && (
        <Alert
          type="warning"
          showIcon
          style={{ marginBottom: 16 }}
          message={`以下证券行情更新失败：${failedCodes.join("、")}`}
          closable
          onClose={() => setFailedCodes([])}
        />
      )}

      <Card>
        <Space style={{ marginBottom: 16 }} wrap>
          <Button type="primary" icon={<PlusOutlined />} onClick={() => setAddOpen(true)}>添加持仓</Button>
          <Button icon={<SyncOutlined />} loading={refreshing} onClick={handleRefresh}>刷新行情</Button>
        </Space>

        <Table<Holding>
          rowKey="id"
          loading={loading}
          dataSource={holdings}
          pagination={{ pageSize: 10 }}
          locale={{ emptyText: "暂无持仓，点击「添加持仓」开始追踪投资" }}
          columns={[
            { title: "代码", dataIndex: "code", width: 90 },
            { title: "名称", dataIndex: "name" },
            {
              title: "类型", dataIndex: "type", width: 80,
              render: (t: string) => t === "fund" ? <Tag color="blue">基金</Tag> : <Tag color="purple">股票</Tag>,
            },
            { title: "数量", dataIndex: "quantity" },
            { title: "成本", dataIndex: "cost_price", render: (v: number) => v.toFixed(3) },
            { title: "现价", dataIndex: "current_price", render: (v: number) => v.toFixed(3) },
            { title: "市值", dataIndex: "market_value", render: (v: number) => formatMoney(v) },
            {
              title: "盈亏",
              render: (_, r) => (
                <span className={r.profit >= 0 ? "profit-positive" : "profit-negative"}>
                  {formatMoney(r.profit)} ({formatPercent(r.profit_rate)})
                </span>
              ),
            },
            { title: "更新", dataIndex: "updated_at", width: 160 },
            {
              title: "操作", width: 140,
              render: (_, r) => (
                <Space>
                  <Button type="link" size="small" icon={<LineChartOutlined />} onClick={() => setKlineHolding(r)}>K线</Button>
                  <Button type="link" size="small" icon={<EditOutlined />} onClick={() => { setEditing(r); editForm.setFieldsValue({ quantity: r.quantity, cost_price: r.cost_price }); setEditOpen(true); }}>编辑</Button>
                  <Popconfirm title="确认删除？" onConfirm={async () => { await api.deleteHolding(r.id); message.success("已删除"); loadData(); }}>
                    <Button type="text" danger size="small" icon={<DeleteOutlined />} />
                  </Popconfirm>
                </Space>
              ),
            },
          ]}
        />
      </Card>

      <Modal title="添加持仓" open={addOpen} onCancel={() => setAddOpen(false)} onOk={handleAdd} destroyOnClose>
        <Form form={addForm} layout="vertical" initialValues={{ type: "stock", quantity: 100, cost_price: 10 }}>
          <Form.Item name="type" label="类型" rules={[{ required: true }]}>
            <Select options={[{ label: "股票", value: "stock" }, { label: "基金", value: "fund" }]} />
          </Form.Item>
          <Form.Item name="code" label="代码" rules={[{ required: true }]} extra="股票如 600519，基金如 000001">
            <Input />
          </Form.Item>
          <Form.Item name="name" label="名称"><Input placeholder="可选" /></Form.Item>
          <Form.Item name="quantity" label="数量" rules={[{ required: true }]}>
            <InputNumber min={0.0001} precision={4} style={{ width: "100%" }} />
          </Form.Item>
          <Form.Item name="cost_price" label="成本价" rules={[{ required: true }]}>
            <InputNumber min={0.0001} precision={4} style={{ width: "100%" }} prefix="¥" />
          </Form.Item>
        </Form>
      </Modal>

      <Modal title={`编辑 ${editing?.name}`} open={editOpen} onCancel={() => setEditOpen(false)} onOk={handleEdit} destroyOnClose>
        <Form form={editForm} layout="vertical">
          <Form.Item name="quantity" label="持有数量" rules={[{ required: true }]}>
            <InputNumber min={0.0001} precision={4} style={{ width: "100%" }} />
          </Form.Item>
          <Form.Item name="cost_price" label="成本价" rules={[{ required: true }]}>
            <InputNumber min={0.0001} precision={4} style={{ width: "100%" }} prefix="¥" />
          </Form.Item>
        </Form>
      </Modal>

      <Modal
        title={klineHolding ? `${klineHolding.name}（${klineHolding.code}）` : "K线"}
        open={!!klineHolding}
        onCancel={() => setKlineHolding(null)}
        footer={null}
        width={960}
        destroyOnClose
      >
        {klineHolding && <KlineChart holding={klineHolding} />}
      </Modal>
    </div>
  );
}
