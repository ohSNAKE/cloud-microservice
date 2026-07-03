import { useCallback, useEffect, useMemo, useState } from "react";
import {
  Alert,
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
import {
  DeleteOutlined,
  EditOutlined,
  LineChartOutlined,
  PlusOutlined,
  SyncOutlined,
} from "@ant-design/icons";
import ReactECharts from "echarts-for-react";
import { api, formatMoney, formatPercent } from "../api";
import EmptyPlaceholder from "../components/layout/EmptyPlaceholder";
import PageHeader from "../components/layout/PageHeader";
import PageLoader from "../components/layout/PageLoader";
import KlineChart from "../components/KlineChart";
import { useTheme } from "../context/ThemeContext";
import type { Holding } from "../types";

export default function HoldingsPage() {
  const { chartColors } = useTheme();
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [holdings, setHoldings] = useState<Holding[]>([]);
  const [failedCodes, setFailedCodes] = useState<string[]>([]);
  const [addOpen, setAddOpen] = useState(false);
  const [editOpen, setEditOpen] = useState(false);
  const [klineHolding, setKlineHolding] = useState<Holding | null>(null);
  const [editing, setEditing] = useState<Holding | null>(null);
  const [addForm] = Form.useForm();
  const [editForm] = Form.useForm();
  const [lookingUpName, setLookingUpName] = useState(false);

  const lookupName = async (code: string, type: string) => {
    const trimmed = code.trim();
    if (!trimmed) return;
    setLookingUpName(true);
    try {
      const name = await api.lookupHoldingName(trimmed, type);
      addForm.setFieldValue("name", name);
    } catch {
      // 查不到名称时保持用户输入或留空，提交时后端会兜底
    } finally {
      setLookingUpName(false);
    }
  };

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

  const allocationOption = useMemo(
    () => ({
      tooltip: { trigger: "item" },
      color: chartColors.palette,
      series: [
        {
          type: "pie",
          radius: ["40%", "65%"],
          data: holdings.map((h) => ({ name: h.name || h.code, value: h.market_value })),
        },
      ],
    }),
    [holdings, chartColors],
  );

  if (loading) {
    return <PageLoader tip="加载持仓数据..." />;
  }

  return (
    <div>
      <PageHeader
        actions={
          <>
            <Button type="primary" icon={<PlusOutlined />} onClick={() => setAddOpen(true)}>
              添加持仓
            </Button>
            <Button icon={<SyncOutlined />} loading={refreshing} onClick={handleRefresh}>
              刷新行情
            </Button>
          </>
        }
      />

      <div
        className={holdings.length > 0 ? "content-grid content-grid--2" : undefined}
        style={{ marginBottom: 16 }}
      >
        <Card className="stat-card content-grid__item">
          <div className="summary-strip">
            <span className="summary-strip__item">
              总市值：<strong>{formatMoney(totalMarketValue)}</strong>
            </span>
            <span className="summary-strip__item">
              总盈亏：
              <strong className={totalProfit >= 0 ? "profit-positive" : "profit-negative"}>
                {formatMoney(totalProfit)}
              </strong>
            </span>
            <span className="summary-strip__item">
              持仓数量：<strong>{holdings.length} 个</strong>
            </span>
          </div>
        </Card>
        {holdings.length > 0 && (
          <Card className="stat-card content-grid__item" title="组合占比">
            <ReactECharts option={allocationOption} style={{ height: 120 }} />
          </Card>
        )}
      </div>

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

      <Card className="stat-card">
        {holdings.length === 0 ? (
          <EmptyPlaceholder description="暂无持仓，添加股票或基金开始追踪投资">
            <Button type="primary" icon={<PlusOutlined />} onClick={() => setAddOpen(true)}>
              添加第一笔持仓
            </Button>
          </EmptyPlaceholder>
        ) : (
          <Table<Holding>
            className="holdings-table"
            rowKey="id"
            dataSource={holdings}
            pagination={{ pageSize: 10 }}
            scroll={{ x: 1180 }}
            tableLayout="fixed"
            columns={[
              { title: "代码", dataIndex: "code", width: 88, fixed: "left" },
              {
                title: "名称",
                dataIndex: "name",
                width: 168,
                ellipsis: { showTitle: true },
                onCell: () => ({ className: "holdings-table__name" }),
              },
              {
                title: "类型",
                dataIndex: "type",
                width: 72,
                render: (t: string) =>
                  t === "fund" ? <Tag color="blue">基金</Tag> : <Tag color="purple">股票</Tag>,
              },
              {
                title: "数量",
                dataIndex: "quantity",
                width: 88,
                align: "right",
                render: (v: number) => v.toLocaleString("zh-CN"),
              },
              {
                title: "成本",
                dataIndex: "cost_price",
                width: 80,
                align: "right",
                render: (v: number) => v.toFixed(3),
              },
              {
                title: "现价",
                dataIndex: "current_price",
                width: 80,
                align: "right",
                render: (v: number) => v.toFixed(3),
              },
              {
                title: "市值",
                dataIndex: "market_value",
                width: 112,
                align: "right",
                render: (v: number) => formatMoney(v),
              },
              {
                title: "盈亏",
                width: 148,
                align: "right",
                onCell: () => ({ className: "holdings-table__profit" }),
                render: (_, r) => (
                  <span className={r.profit >= 0 ? "profit-positive" : "profit-negative"}>
                    {formatMoney(r.profit)} ({formatPercent(r.profit_rate)})
                  </span>
                ),
              },
              {
                title: "更新",
                dataIndex: "updated_at",
                width: 152,
                onCell: () => ({ className: "holdings-table__time" }),
              },
              {
                title: "操作",
                width: 168,
                fixed: "right",
                onCell: () => ({ className: "holdings-table__actions" }),
                render: (_, r) => (
                  <Space>
                    <Button
                      type="link"
                      size="small"
                      icon={<LineChartOutlined />}
                      onClick={() => setKlineHolding(r)}
                    >
                      K线
                    </Button>
                    <Button
                      type="link"
                      size="small"
                      icon={<EditOutlined />}
                      onClick={() => {
                        setEditing(r);
                        editForm.setFieldsValue({ quantity: r.quantity, cost_price: r.cost_price });
                        setEditOpen(true);
                      }}
                    >
                      编辑
                    </Button>
                    <Popconfirm
                      title="确认删除？"
                      onConfirm={async () => {
                        await api.deleteHolding(r.id);
                        message.success("已删除");
                        loadData();
                      }}
                    >
                      <Button type="text" danger size="small" icon={<DeleteOutlined />} />
                    </Popconfirm>
                  </Space>
                ),
              },
            ]}
          />
        )}
      </Card>

      <Modal title="添加持仓" open={addOpen} onCancel={() => setAddOpen(false)} onOk={handleAdd} destroyOnClose>
        <Form form={addForm} layout="vertical" initialValues={{ type: "stock", quantity: 100, cost_price: 10 }}>
          <Form.Item name="type" label="类型" rules={[{ required: true }]}>
            <Select options={[{ label: "股票", value: "stock" }, { label: "基金", value: "fund" }]} />
          </Form.Item>
          <Form.Item name="code" label="代码" rules={[{ required: true }]} extra="股票如 600519，基金如 000001 或 513180">
            <Input
              onBlur={(e) => {
                const type = addForm.getFieldValue("type") ?? "stock";
                void lookupName(e.target.value, type);
              }}
            />
          </Form.Item>
          <Form.Item name="name" label="名称">
            <Input placeholder="留空将自动获取" disabled={lookingUpName} />
          </Form.Item>
          <Form.Item name="quantity" label="数量" rules={[{ required: true }]}>
            <InputNumber min={0.0001} precision={4} style={{ width: "100%" }} />
          </Form.Item>
          <Form.Item name="cost_price" label="成本价" rules={[{ required: true }]}>
            <InputNumber min={0.0001} precision={4} style={{ width: "100%" }} prefix="¥" />
          </Form.Item>
        </Form>
      </Modal>

      <Modal
        title={`编辑 ${editing?.name}`}
        open={editOpen}
        onCancel={() => setEditOpen(false)}
        onOk={handleEdit}
        destroyOnClose
      >
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
