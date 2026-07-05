import { useCallback, useEffect, useState } from "react";
import {
  Button,
  Card,
  Col,
  Form,
  InputNumber,
  Row,
  Select,
  Switch,
  Typography,
  message,
} from "antd";
import { api } from "../../api";
import type { Account, Category, RecurringRule, UpdateRecurringRule } from "../../types";

interface SalaryFormValues {
  enabled: boolean;
  amount: number;
  account_id: number;
  day_of_month: number;
}

function findSalaryRule(rules: RecurringRule[]) {
  return rules.find(
    (r) =>
      r.type === "income" &&
      (r.category_name === "工资" || r.note.includes("工资") || r.note === "月工资"),
  );
}

export default function SalarySettingsCard() {
  const [form] = Form.useForm<SalaryFormValues>();
  const [loading, setLoading] = useState(false);
  const [initialLoading, setInitialLoading] = useState(true);
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [salaryCategoryId, setSalaryCategoryId] = useState<number | null>(null);
  const [ruleId, setRuleId] = useState<number | null>(null);
  const enabled = Form.useWatch("enabled", form);

  const loadData = useCallback(async () => {
    setInitialLoading(true);
    try {
      const [accountList, incomeCats, rules] = await Promise.all([
        api.listAccounts(),
        api.listCategories("income"),
        api.listRecurringRules(),
      ]);
      setAccounts(accountList.filter((a) => a.type !== "broker"));

      const salaryCat = incomeCats.find((c: Category) => c.name === "工资");
      setSalaryCategoryId(salaryCat?.id ?? null);

      const salaryRule = findSalaryRule(rules);
      setRuleId(salaryRule?.id ?? null);

      form.setFieldsValue({
        enabled: salaryRule?.enabled ?? false,
        amount: salaryRule?.amount ?? undefined,
        account_id: salaryRule?.account_id ?? undefined,
        day_of_month: salaryRule?.day_of_month ?? 10,
      });
    } finally {
      setInitialLoading(false);
    }
  }, [form]);

  useEffect(() => {
    loadData();
  }, [loadData]);

  const handleSave = async () => {
    if (!salaryCategoryId) {
      message.error("未找到「工资」收入分类，请检查数据库初始化");
      return;
    }

    setLoading(true);
    try {
      const values = await form.validateFields();
      const payload: UpdateRecurringRule = {
        type: "income",
        amount: values.amount,
        category_id: salaryCategoryId,
        account_id: values.account_id,
        note: "月工资",
        day_of_month: values.day_of_month,
      };

      if (values.enabled) {
        if (ruleId) {
          await api.updateRecurringRule(ruleId, payload);
          await api.toggleRecurringRule(ruleId, true);
        } else {
          const created = await api.addRecurringRule(payload);
          setRuleId(created.id);
          await api.toggleRecurringRule(created.id, true);
        }
        message.success("工资设置已保存，将在每月指定日期自动记账");
      } else if (ruleId) {
        await api.toggleRecurringRule(ruleId, false);
        message.success("已关闭自动记工资");
      } else {
        message.info("未启用自动记工资");
      }
      await loadData();
    } catch (e: unknown) {
      if (e && typeof e === "object" && "errorFields" in e) {
        message.error("请填写完整的工资信息");
      } else {
        message.error(String(e));
      }
    } finally {
      setLoading(false);
    }
  };

  return (
    <Card className="stat-card" title="月薪设置" loading={initialLoading}>
      <Typography.Paragraph type="secondary" style={{ marginBottom: 16 }}>
        设置每月工资金额与到账日期，应用启动时会自动记一笔收入（可在「记账 → 周期记账」查看）
      </Typography.Paragraph>
      <Form form={form} layout="vertical" initialValues={{ day_of_month: 10, enabled: false }}>
        <Form.Item name="enabled" label="启用自动记工资" valuePropName="checked">
          <Switch />
        </Form.Item>
        <Row gutter={16}>
          <Col xs={24} sm={12}>
            <Form.Item
              name="amount"
              label="月工资金额"
              rules={enabled ? [{ required: true, message: "请输入工资金额" }] : []}
            >
              <InputNumber
                min={0.01}
                precision={2}
                prefix="¥"
                style={{ width: "100%" }}
                disabled={!enabled}
                placeholder="如 8000"
              />
            </Form.Item>
          </Col>
          <Col xs={24} sm={12}>
            <Form.Item
              name="day_of_month"
              label="每月到账日"
              rules={enabled ? [{ required: true, message: "请选择日期" }] : []}
              extra="1–28 日，避免月底日期差异"
            >
              <InputNumber min={1} max={28} style={{ width: "100%" }} disabled={!enabled} addonAfter="日" />
            </Form.Item>
          </Col>
        </Row>
        <Form.Item
          name="account_id"
          label="到账账户"
          rules={enabled ? [{ required: true, message: "请选择账户" }] : []}
        >
          <Select
            placeholder="选择工资到账的银行卡或支付宝"
            disabled={!enabled}
            options={accounts.map((a) => ({ label: a.name, value: a.id }))}
          />
        </Form.Item>
        {accounts.length === 0 && enabled && (
          <Typography.Text type="warning" style={{ display: "block", marginBottom: 12 }}>
            请先在「我的资产」登记到账账户
          </Typography.Text>
        )}
        <Button type="primary" loading={loading} onClick={handleSave}>
          保存工资设置
        </Button>
      </Form>
    </Card>
  );
}
