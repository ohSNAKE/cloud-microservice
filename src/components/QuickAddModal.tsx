import { useEffect, useState } from "react";
import {
  Alert,
  Button,
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
import { RobotOutlined, ThunderboltOutlined } from "@ant-design/icons";
import dayjs from "dayjs";
import { api } from "../api";
import type { Account, Category, ParsedTransactionDraft } from "../types";

interface QuickAddModalProps {
  open: boolean;
  onClose: () => void;
  onSuccess?: () => void;
}

export default function QuickAddModal({ open, onClose, onSuccess }: QuickAddModalProps) {
  const [form] = Form.useForm();
  const [confirmForm] = Form.useForm();
  const [categories, setCategories] = useState<Category[]>([]);
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [activeTab, setActiveTab] = useState("smart");
  const [nlText, setNlText] = useState("");
  const [parsing, setParsing] = useState(false);
  const [draft, setDraft] = useState<ParsedTransactionDraft | null>(null);
  const [confirmOpen, setConfirmOpen] = useState(false);
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
      setDraft(null);
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
      setDraft(result);
      confirmForm.setFieldsValue({
        type: result.type,
        amount: result.amount,
        category_id: result.category_id ?? undefined,
        account_id: result.account_id ?? undefined,
        transaction_date: dayjs(result.transaction_date),
        note: result.note,
      });
      setConfirmOpen(true);
    } catch (e) {
      const msg = typeof e === "string" ? e : e instanceof Error ? e.message : String(e);
      message.error(msg || "识别失败，请检查输入或稍后重试");
    } finally {
      setParsing(false);
    }
  };

  const handleConfirmSave = async () => {
    const values = await confirmForm.validateFields();
    await api.addTransaction({
      type: values.type,
      amount: values.amount,
      category_id: values.category_id,
      account_id: values.account_id,
      note: values.note,
      transaction_date: values.transaction_date.format("YYYY-MM-DD"),
    });
    message.success("智能记账成功");
    setConfirmOpen(false);
    onClose();
    onSuccess?.();
  };

  const confirmTxType = Form.useWatch("type", confirmForm) ?? "expense";

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
                    message="用自然语言描述即可，例如：「今天中午微信花了35块买午餐」"
                    description="未配置 AI 时将使用本地规则识别；在设置中开启 AI 可提升识别准确度。"
                  />
                  <Input.TextArea
                    value={nlText}
                    onChange={(e) => setNlText(e.target.value)}
                    placeholder="输入记账内容..."
                    autoSize={{ minRows: 3, maxRows: 6 }}
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
        title="确认记账"
        open={confirmOpen}
        onCancel={() => setConfirmOpen(false)}
        onOk={handleConfirmSave}
        okText="确认入库"
        width={420}
        zIndex={1100}
      >
        {draft && (
          <Space direction="vertical" style={{ width: "100%", marginBottom: 12 }}>
            <Typography.Text type="secondary">原文：{draft.raw_text}</Typography.Text>
            <Space>
              <Tag color={draft.source === "ai" ? "blue" : "default"}>
                {draft.source === "ai" ? "AI 识别" : "规则识别"}
              </Tag>
              <Tag>置信度 {(draft.confidence * 100).toFixed(0)}%</Tag>
            </Space>
          </Space>
        )}
        <Form form={confirmForm} layout="vertical">
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
                .filter((c) => c.type === confirmTxType)
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
            <Input />
          </Form.Item>
        </Form>
      </Modal>
    </>
  );
}
