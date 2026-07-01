import { useEffect, useState } from "react";
import {
  Alert,
  Button,
  Card,
  DatePicker,
  Form,
  Input,
  InputNumber,
  Modal,
  Select,
  Space,
  Tabs,
  Tag,
  Typography,
  message,
} from "antd";
import { DeleteOutlined, RobotOutlined, ThunderboltOutlined } from "@ant-design/icons";
import dayjs from "dayjs";
import { api } from "../api";
import type { Account, Category, ParsedTransactionDraft } from "../types";

interface QuickAddModalProps {
  open: boolean;
  onClose: () => void;
  onSuccess?: () => void;
}

interface ConfirmItem extends ParsedTransactionDraft {
  key: string;
}

export default function QuickAddModal({ open, onClose, onSuccess }: QuickAddModalProps) {
  const [form] = Form.useForm();
  const [categories, setCategories] = useState<Category[]>([]);
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [activeTab, setActiveTab] = useState("smart");
  const [nlText, setNlText] = useState("");
  const [parsing, setParsing] = useState(false);
  const [confirmItems, setConfirmItems] = useState<ConfirmItem[]>([]);
  const [batchMeta, setBatchMeta] = useState<{ raw_text: string; source: string } | null>(null);
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [saving, setSaving] = useState(false);
  const txType = Form.useWatch("type", form) ?? "expense";

  useEffect(() => {
    if (!open) return;
    (async () => {
      const [expenseCats, incomeCats, accountList] = await Promise.all([
        api.listCategories("expense"),
        api.listCategories("income"),
        api.listAccounts(),
      ]);
      setCategories([...expenseCats, ...incomeCats]);
      setAccounts(accountList);
      form.setFieldsValue({ type: "expense", transaction_date: dayjs(), amount: undefined });
      setNlText("");
      setConfirmItems([]);
      setBatchMeta(null);
      setActiveTab("smart");
    })();
  }, [open, form]);

  const handleFormOk = async () => {
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
    onClose();
    onSuccess?.();
  };

  const handleParse = async () => {
    const text = nlText.trim();
    if (!text) {
      message.warning("请输入记账内容");
      return;
    }
    setParsing(true);
    try {
      const result = await api.parseTransactionNl(text);
      if (result.parse_notice) {
        message.warning(result.parse_notice);
      }
      setBatchMeta({ raw_text: result.raw_text, source: result.source });
      setConfirmItems(
        result.items.map((item, index) => ({
          ...item,
          key: `${Date.now()}-${index}`,
        })),
      );
      setConfirmOpen(true);
    } catch (e) {
      const msg = typeof e === "string" ? e : e instanceof Error ? e.message : String(e);
      message.error(msg || "识别失败，请检查输入或稍后重试");
    } finally {
      setParsing(false);
    }
  };

  const updateConfirmItem = (key: string, patch: Partial<ConfirmItem>) => {
    setConfirmItems((prev) => prev.map((item) => (item.key === key ? { ...item, ...patch } : item)));
  };

  const removeConfirmItem = (key: string) => {
    setConfirmItems((prev) => prev.filter((item) => item.key !== key));
  };

  const handleConfirmSave = async () => {
    if (confirmItems.length === 0) {
      message.warning("没有可保存的记录");
      return;
    }

    for (const [index, item] of confirmItems.entries()) {
      if (!item.amount || item.amount <= 0) {
        message.error(`第 ${index + 1} 条缺少有效金额，请填写`);
        return;
      }
      if (!item.category_id) {
        message.error(`第 ${index + 1} 条缺少分类，请选择`);
        return;
      }
      if (!item.account_id) {
        message.error(`第 ${index + 1} 条缺少账户，请选择`);
        return;
      }
    }

    setSaving(true);
    try {
      for (const item of confirmItems) {
        await api.addTransaction({
          type: item.type,
          amount: item.amount!,
          category_id: item.category_id ?? undefined,
          account_id: item.account_id ?? undefined,
          note: item.note,
          transaction_date: item.transaction_date,
        });
      }
      message.success(`已成功入库 ${confirmItems.length} 条记录`);
      setConfirmOpen(false);
      onClose();
      onSuccess?.();
    } catch (e) {
      message.error(String(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <>
      <Modal
        title="快速记账 (⌘N)"
        open={open}
        onCancel={onClose}
        onOk={activeTab === "form" ? handleFormOk : undefined}
        okText={activeTab === "form" ? "保存" : undefined}
        footer={activeTab === "form" ? undefined : null}
        destroyOnClose
        width={480}
      >
        <Tabs
          activeKey={activeTab}
          onChange={setActiveTab}
          items={[
            {
              key: "smart",
              label: (
                <span>
                  <RobotOutlined /> 智能记账
                </span>
              ),
              children: (
                <Space direction="vertical" style={{ width: "100%" }} size="middle">
                  <Alert
                    type="info"
                    showIcon
                    message="支持单条或多条描述"
                    description="例如：「微信买了报纸；支付宝花了三块钱面包」会识别为多条记录。"
                  />
                  <Input.TextArea
                    value={nlText}
                    onChange={(e) => setNlText(e.target.value)}
                    placeholder="输入一条或多条记账内容..."
                    autoSize={{ minRows: 3, maxRows: 8 }}
                    onPressEnter={(e) => {
                      if (!e.shiftKey) {
                        e.preventDefault();
                        handleParse();
                      }
                    }}
                    autoFocus
                  />
                  <Button
                    type="primary"
                    icon={<ThunderboltOutlined />}
                    loading={parsing}
                    onClick={handleParse}
                    block
                  >
                    识别并确认
                  </Button>
                </Space>
              ),
            },
            {
              key: "form",
              label: "表单记账",
              children: (
                <Form form={form} layout="vertical">
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
                        .filter((c) => c.type === txType)
                        .map((c) => ({ label: `${c.icon} ${c.name}`, value: c.id }))}
                    />
                  </Form.Item>
                  <Form.Item name="account_id" label="账户" rules={[{ required: true }]}>
                    <Select options={accounts.map((a) => ({ label: a.name, value: a.id }))} />
                  </Form.Item>
                  <Form.Item name="transaction_date" label="日期" rules={[{ required: true }]}>
                    <DatePicker style={{ width: "100%" }} />
                  </Form.Item>
                  <Form.Item name="note" label="备注">
                    <Input placeholder="可选" />
                  </Form.Item>
                </Form>
              ),
            },
          ]}
        />
      </Modal>

      <Modal
        title={`确认记账（${confirmItems.length} 条）`}
        open={confirmOpen}
        onCancel={() => setConfirmOpen(false)}
        onOk={handleConfirmSave}
        okText={`确认入库 ${confirmItems.length} 条`}
        confirmLoading={saving}
        width={560}
        zIndex={1100}
      >
        {batchMeta && (
          <Space direction="vertical" style={{ width: "100%", marginBottom: 12 }}>
            <Typography.Text type="secondary">原文：{batchMeta.raw_text}</Typography.Text>
            <Tag color={batchMeta.source === "ai" ? "blue" : "default"}>
              {batchMeta.source === "ai" ? "AI 识别" : "规则识别"}
            </Tag>
          </Space>
        )}

        <Space direction="vertical" style={{ width: "100%" }} size="middle">
          {confirmItems.map((item, index) => (
            <Card
              key={item.key}
              size="small"
              title={`第 ${index + 1} 条`}
              extra={
                confirmItems.length > 1 ? (
                  <Button
                    type="text"
                    danger
                    icon={<DeleteOutlined />}
                    onClick={() => removeConfirmItem(item.key)}
                  />
                ) : null
              }
            >
              <Typography.Text type="secondary" style={{ display: "block", marginBottom: 8 }}>
                {item.raw_text}
              </Typography.Text>
              {item.parse_notice && (
                <Alert type="warning" message={item.parse_notice} showIcon style={{ marginBottom: 8 }} />
              )}
              <Space direction="vertical" style={{ width: "100%" }}>
                <Select
                  value={item.type}
                  style={{ width: "100%" }}
                  onChange={(type) =>
                    updateConfirmItem(item.key, { type, category_id: null, category_name: null })
                  }
                  options={[
                    { label: "支出", value: "expense" },
                    { label: "收入", value: "income" },
                  ]}
                />
                <InputNumber
                  min={0.01}
                  precision={2}
                  style={{ width: "100%" }}
                  prefix="¥"
                  placeholder="金额"
                  value={item.amount ?? undefined}
                  onChange={(amount) => updateConfirmItem(item.key, { amount: amount ?? null })}
                />
                <Select
                  placeholder="分类"
                  style={{ width: "100%" }}
                  value={item.category_id ?? undefined}
                  onChange={(category_id) => updateConfirmItem(item.key, { category_id })}
                  options={categories
                    .filter((c) => c.type === item.type)
                    .map((c) => ({ label: `${c.icon} ${c.name}`, value: c.id }))}
                />
                <Select
                  placeholder="账户"
                  style={{ width: "100%" }}
                  value={item.account_id ?? undefined}
                  onChange={(account_id) => updateConfirmItem(item.key, { account_id })}
                  options={accounts.map((a) => ({ label: a.name, value: a.id }))}
                />
                <DatePicker
                  style={{ width: "100%" }}
                  value={dayjs(item.transaction_date)}
                  onChange={(d) =>
                    updateConfirmItem(item.key, {
                      transaction_date: d?.format("YYYY-MM-DD") ?? item.transaction_date,
                    })
                  }
                />
                <Input
                  placeholder="备注"
                  value={item.note}
                  onChange={(e) => updateConfirmItem(item.key, { note: e.target.value })}
                />
              </Space>
            </Card>
          ))}
        </Space>
      </Modal>
    </>
  );
}
