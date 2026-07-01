import { useEffect, useState } from "react";
import { DatePicker, Form, Input, InputNumber, Modal, Select, message } from "antd";
import dayjs from "dayjs";
import { api } from "../api";
import type { Account, Category } from "../types";

interface QuickAddModalProps {
  open: boolean;
  onClose: () => void;
  onSuccess?: () => void;
}

export default function QuickAddModal({ open, onClose, onSuccess }: QuickAddModalProps) {
  const [form] = Form.useForm();
  const [categories, setCategories] = useState<Category[]>([]);
  const [accounts, setAccounts] = useState<Account[]>([]);
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
    })();
  }, [open, form]);

  const handleOk = async () => {
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

  return (
    <Modal
      title="快速记账 (⌘N)"
      open={open}
      onCancel={onClose}
      onOk={handleOk}
      destroyOnClose
      width={420}
    >
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
          <InputNumber min={0.01} precision={2} style={{ width: "100%" }} prefix="¥" autoFocus />
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
    </Modal>
  );
}
