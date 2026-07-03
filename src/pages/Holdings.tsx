import { useCallback, useEffect, useMemo, useState } from "react";
import {
  Alert,
  App,
  Button,
  Card,
  Col,
  Form,
  Input,
  InputNumber,
  Modal,
  Row,
  Select,
  Space,
  Tag,
  Typography,
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
  const { modal, message } = App.useApp();
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
      /* 查不到名称时保持用户输入或留空 */
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

  const confirmDelete = (holding: Holding) => {
    modal.confirm({
      title: `删除「${holding.name || holding.code}」？`,
      content: "删除后不可恢复。",
      okText: "删除",
      okType: "danger",
      cancelText: "取消",
      centered: true,
      onOk: async () => {
        await api.deleteHolding(holding.id);
        message.success("已删除");
        loadData();
      },
    });
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

  const renderHoldingCard = (holding: Holding) => (
    <Card key={holding.id} className="stat-card holding-card" hoverable>
      <div className="holding-card__head">
        <span className="holding-card__code">{holding.code}</span>
        <div className="holding-card__meta">
          <Typography.Text strong ellipsis={{ tooltip: holding.name || holding.code }}>
            {holding.name || holding.code}
          </Typography.Text>
          <Tag color={holding.type === "fund" ? "blue" : "purple"} style={{ marginTop: 4 }}>
            {holding.type === "fund" ? "基金" : "股票"}
          </Tag>
        </div>
      </div>

      <div className="holding-card__value">{formatMoney(holding.market_value)}</div>
      <div className={`holding-card__profit ${holding.profit >= 0 ? "profit-positive" : "profit-negative"}`}>
        {formatMoney(holding.profit)} · {formatPercent(holding.profit_rate)}
      </div>

      <div className="holding-card__stats">
        <div className="holding-card__stat">
          <span className="holding-card__stat-label">数量</span>
          <span>{holding.quantity.toLocaleString("zh-CN")}</span>
        </div>
        <div className="holding-card__stat">
          <span className="holding-card__stat-label">成本</span>
          <span>{holding.cost_price.toFixed(3)}</span>
        </div>
        <div className="holding-card__stat">
          <span className="holding-card__stat-label">现价</span>
          <span>{holding.current_price.toFixed(3)}</span>
        </div>
      </div>

      <Typography.Text type="secondary" className="holding-card__time">
        更新 {holding.updated_at}
      </Typography.Text>

      <Space size={4} wrap className="holding-card__actions">
        <Button type="link" size="small" icon={<LineChartOutlined />} onClick={() => setKlineHolding(holding)}>
          K线
        </Button>
        <Button
          type="link"
          size="small"
          icon={<EditOutlined />}
          onClick={() => {
            setEditing(holding);
            editForm.setFieldsValue({ quantity: holding.quantity, cost_price: holding.cost_price });
            setEditOpen(true);
          }}
        >
          编辑
        </Button>
        <Button type="link" size="small" danger icon={<DeleteOutlined />} onClick={() => confirmDelete(holding)}>
          删除
        </Button>
      </Space>
    </Card>
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

      {holdings.length === 0 ? (
        <Card className="stat-card">
          <EmptyPlaceholder description="暂无持仓，添加股票或基金开始追踪投资">
            <Button type="primary" icon={<PlusOutlined />} onClick={() => setAddOpen(true)}>
              添加第一笔持仓
            </Button>
          </EmptyPlaceholder>
        </Card>
      ) : (
        <Row gutter={[16, 16]} className="holding-grid">
          {holdings.map((holding) => (
            <Col key={holding.id} xs={24} sm={12} lg={8} xl={6} className="holding-grid__col">
              {renderHoldingCard(holding)}
            </Col>
          ))}
        </Row>
      )}

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
