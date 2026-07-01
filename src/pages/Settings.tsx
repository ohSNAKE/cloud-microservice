import { useEffect, useState } from "react";
import { Button, Card, Form, InputNumber, Switch, message } from "antd";
import { api } from "../api";
import type { Settings } from "../types";

export default function SettingsPage() {
  const [form] = Form.useForm<Settings>();
  const [loading, setLoading] = useState(false);
  const [lastSyncAt, setLastSyncAt] = useState<string | null>(null);

  useEffect(() => {
    (async () => {
      const [settings, syncAt] = await Promise.all([api.getSettings(), api.getLastSyncAt()]);
      form.setFieldsValue(settings);
      setLastSyncAt(syncAt);
    })();
  }, [form]);

  const handleSave = async () => {
    const values = await form.validateFields();
    setLoading(true);
    try {
      await api.updateSettings(values);
      message.success("设置已保存");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div>
      <div className="page-header">
        <h2>设置</h2>
        <p>配置行情自动更新与基础参数</p>
      </div>

      <Card style={{ maxWidth: 560 }}>
        <Form form={form} layout="vertical">
          <Form.Item
            name="quote_update_enabled"
            label="自动更新行情"
            valuePropName="checked"
          >
            <Switch />
          </Form.Item>
          <Form.Item
            name="quote_update_interval"
            label="更新间隔（分钟）"
            rules={[{ required: true }]}
          >
            <InputNumber min={5} max={240} style={{ width: "100%" }} />
          </Form.Item>
          <Form.Item label="上次同步时间">
            <span>{lastSyncAt ?? "尚未同步"}</span>
          </Form.Item>
        </Form>
        <Button type="primary" loading={loading} onClick={handleSave}>
          保存设置
        </Button>
      </Card>
    </div>
  );
}
