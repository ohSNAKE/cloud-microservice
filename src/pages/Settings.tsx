import { useEffect, useState } from "react";
import { Button, Card, Form, Input, InputNumber, Switch, Typography, Upload, message } from "antd";
import { DownloadOutlined, UploadOutlined } from "@ant-design/icons";
import { api } from "../api";
import type { Settings } from "../types";

export default function SettingsPage() {
  const [form] = Form.useForm<Settings>();
  const [loading, setLoading] = useState(false);
  const [lastSyncAt, setLastSyncAt] = useState<string | null>(null);
  const [dbPath, setDbPath] = useState("");
  const aiEnabled = Form.useWatch("ai_enabled", form);

  useEffect(() => {
    (async () => {
      const [settings, syncAt, path] = await Promise.all([
        api.getSettings(),
        api.getLastSyncAt(),
        api.getDbPath(),
      ]);
      form.setFieldsValue(settings);
      setLastSyncAt(syncAt);
      setDbPath(path);
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

  const handleExport = async () => {
    try {
      const json = await api.exportData();
      const blob = new Blob([json], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const link = document.createElement("a");
      link.href = url;
      link.download = `财记备份_${new Date().toISOString().slice(0, 10)}.json`;
      link.click();
      URL.revokeObjectURL(url);
      message.success("备份已下载");
    } catch (e) {
      message.error(`导出失败：${e}`);
    }
  };

  const handleImport = async (file: File) => {
    try {
      const text = await file.text();
      await api.importData(text);
      message.success("数据已恢复，即将刷新");
      window.location.reload();
    } catch (e) {
      message.error(`导入失败：${e}`);
    }
    return false;
  };

  return (
    <div>
      <div className="page-header">
        <h2>设置</h2>
        <p>行情同步、智能记账、数据备份与应用配置</p>
      </div>

      <Form form={form} layout="vertical">
        <Card title="智能记账" style={{ maxWidth: 560, marginBottom: 16 }}>
          <Form.Item name="ai_enabled" label="启用 AI 识别" valuePropName="checked">
            <Switch />
          </Form.Item>
          <Form.Item
            name="ai_api_key"
            label="API Key"
            extra="支持 OpenAI 兼容接口（默认 PinCC）。Key 仅存本地数据库，不会上传。"
          >
            <Input.Password placeholder="sk-..." disabled={!aiEnabled} />
          </Form.Item>
          <Form.Item name="ai_api_base" label="API 地址">
            <Input placeholder="https://v2.pincc.ai/v1" disabled={!aiEnabled} />
          </Form.Item>
          <Form.Item name="ai_model" label="模型">
            <Input placeholder="gpt-4o-mini" disabled={!aiEnabled} />
          </Form.Item>
        </Card>

        <Card title="行情设置" style={{ maxWidth: 560, marginBottom: 16 }}>
          <Form.Item name="quote_update_enabled" label="自动更新行情" valuePropName="checked">
            <Switch />
          </Form.Item>
          <Form.Item name="refresh_on_startup" label="启动时刷新行情" valuePropName="checked">
            <Switch />
          </Form.Item>
          <Form.Item name="quote_update_interval" label="更新间隔（分钟）" rules={[{ required: true }]}>
            <InputNumber min={5} max={240} style={{ width: "100%" }} />
          </Form.Item>
          <Form.Item label="上次同步">{lastSyncAt ?? "尚未同步"}</Form.Item>
        </Card>
      </Form>

      <Button type="primary" loading={loading} onClick={handleSave} style={{ marginBottom: 16 }}>
        保存设置
      </Button>

      <Card title="数据管理" style={{ maxWidth: 560, marginBottom: 16 }}>
        <Typography.Paragraph type="secondary" copyable={dbPath ? { text: dbPath } : undefined}>
          数据库路径：{dbPath || "加载中..."}
        </Typography.Paragraph>
        <Typography.Paragraph type="secondary">
          建议定期导出备份。导入会覆盖当前全部数据，请谨慎操作。
        </Typography.Paragraph>
        <Button icon={<DownloadOutlined />} onClick={handleExport} style={{ marginRight: 8 }}>
          导出备份
        </Button>
        <Upload accept=".json" showUploadList={false} beforeUpload={handleImport}>
          <Button icon={<UploadOutlined />}>导入恢复</Button>
        </Upload>
      </Card>
    </div>
  );
}
