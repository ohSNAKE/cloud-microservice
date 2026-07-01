import { useCallback, useEffect, useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";
import {
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
  Typography,
  message,
} from "antd";
import { EditOutlined, LineChartOutlined, PlusOutlined } from "@ant-design/icons";
import { api, accountTypeIcon, accountTypeLabel, formatMoney } from "../api";
import EmptyPlaceholder from "../components/layout/EmptyPlaceholder";
import PageHeader from "../components/layout/PageHeader";
import PageLoader from "../components/layout/PageLoader";
import { ACCOUNT_TYPES } from "../types";
import type { Account, NewAccount, UpdateAccount } from "../types";

export default function AssetsPage() {
  const navigate = useNavigate();
  const [loading, setLoading] = useState(true);
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [modalOpen, setModalOpen] = useState(false);
  const [balanceModalOpen, setBalanceModalOpen] = useState(false);
  const [editing, setEditing] = useState<Account | null>(null);
  const [form] = Form.useForm<NewAccount & { balance: number }>();
  const [balanceForm] = Form.useForm<{ balance: number }>();

  const loadData = useCallback(async () => {
    setLoading(true);
    try {
      setAccounts(await api.listAccounts());
    } finally {
      setLoading(false);
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
    try {
      await api.deleteAccount(id);
      message.success("已删除");
      loadData();
    } catch (e) {
      message.error(String(e));
    }
  };

  if (loading) {
    return <PageLoader tip="加载资产..." />;
  }

  const renderAccountCard = (account: Account) => (
    <Card key={account.id} className="stat-card asset-card" hoverable>
      <div className="asset-card__head">
        <span className="asset-card__icon" aria-hidden>
          {accountTypeIcon(account.type)}
        </span>
        <div className="asset-card__meta">
          <Typography.Text strong>{account.name}</Typography.Text>
          <Typography.Text type="secondary" style={{ fontSize: 12 }}>
            {accountTypeLabel(account.type)}
          </Typography.Text>
        </div>
      </div>
      <div className={`asset-card__balance ${account.balance < 0 ? "is-negative" : ""}`}>
        {formatMoney(account.balance)}
      </div>
      <Space size={4} wrap style={{ marginTop: 12 }}>
        <Button type="link" size="small" onClick={() => openBalanceEdit(account)}>
          更新余额
        </Button>
        <Button type="link" size="small" icon={<EditOutlined />} onClick={() => openEdit(account)}>
          编辑
        </Button>
        <Popconfirm title="确认删除该账户？" onConfirm={() => deleteAccount(account.id)}>
          <Button type="link" size="small" danger>
            删除
          </Button>
        </Popconfirm>
      </Space>
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
        <Row gutter={[16, 16]}>
          {personalAccounts.map((account) => (
            <Col key={account.id} xs={24} sm={12} lg={8} xl={6}>
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
          <Row gutter={[16, 16]}>
            {brokerAccounts.map((account) => (
              <Col key={account.id} xs={24} sm={12} lg={8} xl={6}>
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
