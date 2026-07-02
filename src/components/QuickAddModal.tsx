import { useEffect, useState } from "react";
import { flushSync } from "react-dom";
import {
  Button,
  DatePicker,
  Divider,
  Empty,
  Form,
  Input,
  InputNumber,
  Modal,
  Row,
  Col,
  Select,
  Skeleton,
  Space,
  Spin,
  Tabs,
  Tag,
  Typography,
  message,
} from "antd";
import { LoadingOutlined, DeleteOutlined, RobotOutlined, ThunderboltOutlined } from "@ant-design/icons";
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

function DraftItemEditor({
  item,
  index,
  categories,
  accounts,
  removable,
  onChange,
  onRemove,
}: {
  item: ConfirmItem;
  index: number;
  categories: Category[];
  accounts: Account[];
  removable: boolean;
  onChange: (patch: Partial<ConfirmItem>) => void;
  onRemove: () => void;
}) {
  return (
    <div
      className="draft-item"
    >
      <div className="draft-item__header">
        <Typography.Text strong style={{ fontSize: 13 }}>
          #{index + 1}
        </Typography.Text>
        <Typography.Text
          type="secondary"
          ellipsis
          style={{ flex: 1, fontSize: 12 }}
          title={item.raw_text}
        >
          {item.raw_text}
        </Typography.Text>
        {removable && (
          <Button type="text" size="small" danger icon={<DeleteOutlined />} onClick={onRemove} />
        )}
      </div>

      {item.parse_notice && (
        <Typography.Text type="warning" style={{ fontSize: 12, display: "block", marginBottom: 8 }}>
          {item.parse_notice}
        </Typography.Text>
      )}

      <Row gutter={[8, 8]}>
        <Col span={8}>
          <Select
            size="small"
            value={item.type}
            style={{ width: "100%" }}
            onChange={(type) => onChange({ type, category_id: null, category_name: null })}
            options={[
              { label: "支出", value: "expense" },
              { label: "收入", value: "income" },
            ]}
          />
        </Col>
        <Col span={8}>
          <InputNumber
            size="small"
            min={0.01}
            precision={2}
            style={{ width: "100%" }}
            prefix="¥"
            placeholder="金额"
            value={item.amount ?? undefined}
            onChange={(amount) => onChange({ amount: amount ?? null })}
          />
        </Col>
        <Col span={8}>
          <DatePicker
            size="small"
            style={{ width: "100%" }}
            value={dayjs(item.transaction_date)}
            onChange={(d) =>
              onChange({ transaction_date: d?.format("YYYY-MM-DD") ?? item.transaction_date })
            }
          />
        </Col>
        <Col span={12}>
          <Select
            size="small"
            placeholder="分类"
            style={{ width: "100%" }}
            value={item.category_id ?? undefined}
            onChange={(category_id) => onChange({ category_id })}
            options={categories
              .filter((c) => c.type === item.type)
              .map((c) => ({ label: `${c.icon} ${c.name}`, value: c.id }))}
          />
        </Col>
        <Col span={12}>
          <Select
            size="small"
            placeholder="账户"
            style={{ width: "100%" }}
            value={item.account_id ?? undefined}
            onChange={(account_id) => onChange({ account_id })}
            options={accounts.map((a) => ({ label: a.name, value: a.id }))}
          />
        </Col>
        <Col span={24}>
          <Input
            size="small"
            placeholder="备注"
            value={item.note}
            onChange={(e) => onChange({ note: e.target.value })}
          />
        </Col>
      </Row>
    </div>
  );
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
    flushSync(() => {
      setParsing(true);
      setConfirmItems([]);
      setBatchMeta(null);
    });
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
      message.warning("请先识别记账内容");
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
      onClose();
      onSuccess?.();
    } catch (e) {
      message.error(String(e));
    } finally {
      setSaving(false);
    }
  };

  const smartFooter =
    activeTab === "smart" ? (
      <Space>
        <Button onClick={onClose}>取消</Button>
        <Button
          type="primary"
          loading={saving}
          disabled={confirmItems.length === 0}
          onClick={handleConfirmSave}
        >
          {confirmItems.length > 0 ? `确认入库 ${confirmItems.length} 条` : "确认入库"}
        </Button>
      </Space>
    ) : undefined;

  return (
    <Modal
      className="quick-add-modal"
      title="快速记账 (⌘N)"
      open={open}
      onCancel={onClose}
      onOk={activeTab === "form" ? handleFormOk : undefined}
      okText={activeTab === "form" ? "保存" : undefined}
      footer={activeTab === "smart" ? smartFooter : activeTab === "form" ? undefined : null}
      destroyOnClose
      width={activeTab === "smart" ? 920 : 480}
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
              <Spin spinning={parsing} tip="正在识别，请稍候..." size="large">
                <div className="quick-add-modal__layout">
                  <div className="quick-add-modal__input-panel">
                    <Typography.Text type="secondary" style={{ fontSize: 13 }}>
                      支持多条描述，用分号或「还有」分隔
                    </Typography.Text>
                    <Input.TextArea
                      value={nlText}
                      onChange={(e) => setNlText(e.target.value)}
                      placeholder="例如：微信买了报纸；支付宝花了三块钱面包"
                      autoSize={{ minRows: 10, maxRows: 14 }}
                      disabled={parsing}
                      autoFocus
                    />
                    <Button
                      type="primary"
                      icon={parsing ? <LoadingOutlined spin /> : <ThunderboltOutlined />}
                      loading={parsing}
                      disabled={parsing}
                      onClick={handleParse}
                      block
                    >
                      {parsing ? "正在识别..." : "识别"}
                    </Button>
                  </div>

                  <Divider type="vertical" className="quick-add-modal__divider" />

                  <div className="quick-add-modal__result-panel">
                    <div className="quick-add-modal__result-header">
                      <Typography.Text strong>识别结果</Typography.Text>
                      {parsing ? (
                        <Tag icon={<LoadingOutlined spin />} color="processing">
                          识别中
                        </Tag>
                      ) : (
                        batchMeta && (
                          <Tag color={batchMeta.source === "ai" ? "blue" : "default"}>
                            {batchMeta.source === "ai" ? "AI" : "规则"}
                            {confirmItems.length > 0 ? ` · ${confirmItems.length} 条` : ""}
                          </Tag>
                        )
                      )}
                    </div>

                    <div className={`quick-add-modal__result-scroll${parsing ? " is-loading" : ""}`}>
                      {parsing ? (
                        <div style={{ padding: "16px 8px" }}>
                          <Skeleton active paragraph={{ rows: 5 }} />
                        </div>
                      ) : confirmItems.length === 0 ? (
                        <Empty
                          image={Empty.PRESENTED_IMAGE_SIMPLE}
                          description="识别结果将显示在这里"
                          style={{ marginTop: 80 }}
                        />
                      ) : (
                        <Space direction="vertical" style={{ width: "100%" }} size={10}>
                          {confirmItems.map((item, index) => (
                            <DraftItemEditor
                              key={item.key}
                              item={item}
                              index={index}
                              categories={categories}
                              accounts={accounts}
                              removable={confirmItems.length > 1}
                              onChange={(patch) => updateConfirmItem(item.key, patch)}
                              onRemove={() => removeConfirmItem(item.key)}
                            />
                          ))}
                        </Space>
                      )}
                    </div>
                  </div>
                </div>
              </Spin>
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
  );
}
