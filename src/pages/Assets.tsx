import { useCallback, useEffect, useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";
import {
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
  Tag,
  Tooltip,
  Typography,
} from "antd";
import {
  DeleteOutlined,
  EditOutlined,
  LineChartOutlined,
  PlusOutlined,
  WalletOutlined,
} from "@ant-design/icons";
import { api, accountTypeIcon, accountTypeLabel, formatInvokeError, formatMoney } from "../api";
import EmptyPlaceholder from "../components/layout/EmptyPlaceholder";
import PageHeader from "../components/layout/PageHeader";
import PageLoader from "../components/layout/PageLoader";
import { ACCOUNT_TYPES } from "../types";
import type { Account, NewAccount, UpdateAccount } from "../types";

function accountTypeTagColor(type: string) {
  switch (type) {
    case "cash":
      return "gold";
    case "bank":
      return "blue";
    case "alipay":
      return "cyan";
    case "wechat":
      return "green";
    case "broker":
      return "purple";
    default:
      return "default";
  }
}

export default function AssetsPage() {
  const navigate = useNavigate();
  const { modal, message } = App.useApp();
  const [loading, setLoading] = useState(true);
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [modalOpen, setModalOpen] = useState(false);
  const [balanceModalOpen, setBalanceModalOpen] = useState(false);
  const [editing, setEditing] = useState<Account | null>(null);
  const [form] = Form.useForm<NewAccount & { balance: number }>();
  const [balanceForm] = Form.useForm<{ balance: number }>();

  const loadData = useCallback(async (opts?: { silent?: boolean }) => {
    if (!opts?.silent) setLoading(true);
    try {
      setAccounts(await api.listAccounts());
    } finally {
      if (!opts?.silent) setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadData();
  }, [loadData]);

  const personalAccounts = useMemo(
    () => accounts.filter((a) => a.type !== "broker"),
    [accounts],
  );
  const brokerAccounts = useMemo(
    () => accounts.filter((a) => a.type === "broker"),
    [accounts],
  );

  const liquidTotal = useMemo(
    () => personalAccounts.reduce((sum, a) => sum + a.balance, 0),
    [personalAccounts],
  );
  const brokerTotal = useMemo(
    () => brokerAccounts.reduce((sum, a) => sum + a.balance, 0),
    [brokerAccounts],
  );

  const openCreate = () => {
    setEditing(null);
    form.resetFields();
    form.setFieldsValue({ type: "bank", balance: 0 });
    setModalOpen(true);
  };

  const openEdit = (account: Account) => {
    setEditing(account);
    form.setFieldsValue(account);
    setModalOpen(true);
  };

  const openBalanceEdit = (account: Account) => {
    setEditing(account);
    balanceForm.setFieldsValue({ balance: account.balance });
    setBalanceModalOpen(true);
  };

  const saveAccount = async () => {
    const values = await form.validateFields();
    if (editing) {
      await api.updateAccount(editing.id, values as UpdateAccount);
      message.success("账户已更新");
    } else {
      await api.addAccount(values);
      message.success("账户已添加");
    }
    setModalOpen(false);
    loadData();
  };

  const saveBalance = async () => {
    if (!editing) return;
    const { balance } = await balanceForm.validateFields();
    await api.updateAccount(editing.id, {
      name: editing.name,
      type: editing.type,
      balance,
    });
    message.success("余额已更新");
    setBalanceModalOpen(false);
    loadData();
  };

  const deleteAccount = async (id: number) => {
    await api.deleteAccount(id);
    setAccounts((prev) => prev.filter((a) => a.id !== id));
    message.success("账户已删除");
    await loadData({ silent: true });
  };

  const confirmDeleteAccount = (account: Account) => {
    modal.confirm({
      title: `删除「${account.name}」？`,
      content:
        "删除后不可恢复。若该账户有关联记账或已绑定工资/周期规则，将无法删除并会提示原因。",
      okText: "删除",
      okType: "danger",
      cancelText: "取消",
      centered: true,
      onOk: async () => {
        try {
          await deleteAccount(account.id);
        } catch (e) {
          message.error(formatInvokeError(e));
          throw e;
        }
      },
    });
  };

  if (loading) {
    return <PageLoader tip="加载资产..." />;
  }

  const renderAccountCard = (account: Account) => (
    <Card key={account.id} className="stat-card asset-card" hoverable>
      <div className="asset-card__header">
        <div className="asset-card__title-row">
          <Typography.Text strong className="asset-card__name" ellipsis={{ tooltip: account.name }}>
            {account.name}
          </Typography.Text>
          <Tag
            bordered={false}
            color={accountTypeTagColor(account.type)}
            className="asset-card__tag"
          >
            {accountTypeLabel(account.type)}
          </Tag>
        </div>
        <span className="asset-card__subtitle">
          {accountTypeIcon(account.type)} {accountTypeLabel(account.type)}
        </span>
      </div>

      <div className="asset-card__body">
        <div className={`asset-card__value ${account.balance < 0 ? "is-negative" : ""}`}>
          {formatMoney(account.balance)}
        </div>
      </div>

      <div className="asset-card__stats">
        <div className="asset-card__stat">
          <span className="asset-card__stat-label">类型</span>
          <span className="asset-card__stat-value">{accountTypeLabel(account.type)}</span>
        </div>
        <div className="asset-card__stat">
          <span className="asset-card__stat-label">分类</span>
          <span className="asset-card__stat-value">
            {account.type === "broker" ? "证券" : "个人"}
          </span>
        </div>
        <div className="asset-card__stat">
          <span className="asset-card__stat-label">计入总资产</span>
          <span className="asset-card__stat-value">{account.type === "broker" ? "否" : "是"}</span>
        </div>
      </div>

      <div className="asset-card__footer">
        <Typography.Text type="secondary" className="asset-card__hint">
          {account.type === "broker" ? "不计入可动用资产" : "可动用资产"}
        </Typography.Text>
        <div className="asset-card__actions">
          <Tooltip title="更新余额">
            <Button
              type="text"
              size="small"
              className="asset-card__action-btn"
              icon={<WalletOutlined />}
              onClick={() => openBalanceEdit(account)}
            />
          </Tooltip>
          <Tooltip title="编辑">
            <Button
              type="text"
              size="small"
              className="asset-card__action-btn"
              icon={<EditOutlined />}
              onClick={() => openEdit(account)}
            />
          </Tooltip>
          <Tooltip title="删除">
            <Button
              type="text"
              size="small"
              danger
              className="asset-card__action-btn"
              icon={<DeleteOutlined />}
              onClick={() => confirmDeleteAccount(account)}
            />
          </Tooltip>
        </div>
      </div>
    </Card>
  );

  return (
    <div>
      <PageHeader
        actions={
          <>
            <Button type="primary" icon={<PlusOutlined />} onClick={openCreate}>
              登记账户
            </Button>
            <Button icon={<LineChartOutlined />} onClick={() => navigate("/holdings")}>
              投资持仓
            </Button>
          </>
        }
      />

      <Card className="stat-card" style={{ marginBottom: 16 }}>
        <div className="summary-strip">
          <span className="summary-strip__item">
            可动用资产：<strong>{formatMoney(liquidTotal)}</strong>
          </span>
          {brokerTotal > 0 && (
            <span className="summary-strip__item">
              证券账户余额：<strong>{formatMoney(brokerTotal)}</strong>
              <Typography.Text type="secondary" style={{ fontSize: 12, marginLeft: 6 }}>
                （不计入总资产）
              </Typography.Text>
            </span>
          )}
          <span className="summary-strip__item">
            账户数量：<strong>{accounts.length} 个</strong>
          </span>
        </div>
      </Card>

      <Typography.Title level={5} style={{ marginBottom: 12 }}>
        个人账户
      </Typography.Title>
      <Typography.Paragraph type="secondary" style={{ marginTop: -4, marginBottom: 16 }}>
        登记银行卡、支付宝、微信、现金等余额，用于统计可动用资产
      </Typography.Paragraph>

      {personalAccounts.length === 0 ? (
        <Card className="stat-card">
          <EmptyPlaceholder description="还没有登记任何个人账户">
            <Button type="primary" icon={<PlusOutlined />} onClick={openCreate}>
              登记第一个账户
            </Button>
          </EmptyPlaceholder>
        </Card>
      ) : (
        <Row gutter={[16, 16]} className="asset-grid">
          {personalAccounts.map((account) => (
            <Col key={account.id} xs={24} sm={12} lg={8} xl={8} className="asset-grid__col">
              {renderAccountCard(account)}
            </Col>
          ))}
        </Row>
      )}

      {brokerAccounts.length > 0 && (
        <>
          <Typography.Title level={5} style={{ marginTop: 24, marginBottom: 12 }}>
            证券账户
          </Typography.Title>
          <Row gutter={[16, 16]} className="asset-grid">
            {brokerAccounts.map((account) => (
              <Col key={account.id} xs={24} sm={12} lg={8} xl={8} className="asset-grid__col">
                {renderAccountCard(account)}
              </Col>
            ))}
          </Row>
        </>
      )}

      <Modal
        title={editing ? "编辑账户" : "登记账户"}
        open={modalOpen}
        onCancel={() => setModalOpen(false)}
        onOk={saveAccount}
        destroyOnClose
      >
        <Form form={form} layout="vertical">
          <Form.Item name="name" label="账户名称" rules={[{ required: true, message: "请输入名称" }]}>
            <Input placeholder="如：工资卡、支付宝" />
          </Form.Item>
          <Form.Item name="type" label="账户类型" rules={[{ required: true }]}>
            <Select options={ACCOUNT_TYPES.map((t) => ({ label: `${t.icon} ${t.label}`, value: t.value }))} />
          </Form.Item>
          <Form.Item
            name="balance"
            label="当前余额"
            rules={[{ required: true, message: "请输入余额" }]}
            extra="首次登记时填写当前实际余额，之后记账会自动更新"
          >
            <InputNumber precision={2} style={{ width: "100%" }} prefix="¥" />
          </Form.Item>
        </Form>
      </Modal>

      <Modal
        title={editing ? `更新余额 · ${editing.name}` : "更新余额"}
        open={balanceModalOpen}
        onCancel={() => setBalanceModalOpen(false)}
        onOk={saveBalance}
        destroyOnClose
      >
        <Typography.Paragraph type="secondary">
          直接修改为当前实际余额，不会生成记账流水
        </Typography.Paragraph>
        <Form form={balanceForm} layout="vertical">
          <Form.Item name="balance" label="当前余额" rules={[{ required: true }]}>
            <InputNumber precision={2} style={{ width: "100%" }} prefix="¥" />
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
}
