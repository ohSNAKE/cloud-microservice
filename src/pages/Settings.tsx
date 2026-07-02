import { useEffect, useState } from "react";
import {
  Button,
  Card,
  Col,
  Form,
  Input,
  InputNumber,
  Modal,
  Row,
  Switch,
  Typography,
  Upload,
  message,
} from "antd";
import { DownloadOutlined, UploadOutlined } from "@ant-design/icons";
import { api } from "../api";
import PageHeader from "../components/layout/PageHeader";
import PageLoader from "../components/layout/PageLoader";
import ThemeSwitcher from "../components/theme/ThemeSwitcher";
import { useTheme } from "../context/ThemeContext";
import type { Settings } from "../types";

export default function SettingsPage() {
  const [form] = Form.useForm<Settings>();
  const { mode: themeMode } = useTheme();
  const [loading, setLoading] = useState(false);
  const [initialLoading, setInitialLoading] = useState(true);
  const [lastSyncAt, setLastSyncAt] = useState<string | null>(null);
  const [dbPath, setDbPath] = useState("");
  const [importModalOpen, setImportModalOpen] = useState(false);
  const [pendingImportFile, setPendingImportFile] = useState<File | null>(null);
  const aiEnabled = Form.useWatch("ai_enabled", form);

  useEffect(() => {
    (async () => {
      try {
        const [settings, syncAt, path] = await Promise.all([
          api.getSettings(),
          api.getLastSyncAt(),
          api.getDbPath(),
        ]);
        form.setFieldsValue(settings);
        setLastSyncAt(syncAt);
        setDbPath(path);
      } finally {
        setInitialLoading(false);
      }
    })();
  }, [form]);

  const handleSave = async () => {
    setLoading(true);
    try {
      await form.validateFields();
      const [current, values] = await Promise.all([
        api.getSettings(),
        Promise.resolve(form.getFieldsValue(true) as Partial<Settings>),
      ]);
      const merged: Settings = { ...current, ...values, theme_mode: themeMode };
      await api.updateSettings(merged);
      form.setFieldsValue(merged);
      message.success("设置已保存");
    } catch (e: unknown) {
      if (e && typeof e === "object" && "errorFields" in e) {
        message.error("请检查表单是否填写完整");
      } else {
        message.error(`保存失败：${String(e)}`);
      }
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
      link.download = `穷鬼备份_${new Date().toISOString().slice(0, 10)}.json`;
      link.click();
      URL.revokeObjectURL(url);
      message.success("备份已下载");
    } catch (e) {
      message.error(`导出失败：${e}`);
    }
  };

  const handleImportConfirm = async () => {
    if (!pendingImportFile) return;
    try {
      const text = await pendingImportFile.text();
      await api.importData(text);
      message.success("数据已恢复，即将刷新");
      window.location.reload();
    } catch (e) {
      message.error(`导入失败：${e}`);
    } finally {
      setImportModalOpen(false);
      setPendingImportFile(null);
    }
  };

  const beforeImport = (file: File) => {
    setPendingImportFile(file);
    setImportModalOpen(true);
    return false;
  };

  if (initialLoading) {
    return <PageLoader tip="加载设置..." />;
  }

  return (
    <div className="settings-page">
      <PageHeader />

      <Form form={form} layout="vertical" preserve>
        <Form.Item name="currency" hidden>
          <Input />
        </Form.Item>

        <Row gutter={[16, 16]}>
          <Col xs={24}>
            <Card className="stat-card" title="外观">
              <Typography.Paragraph type="secondary" style={{ marginBottom: 12 }}>
                选择浅色、深色，或跟随系统外观自动切换
              </Typography.Paragraph>
              <ThemeSwitcher block />
            </Card>
          </Col>

          <Col xs={24} lg={12}>
            <Card className="stat-card" title="智能记账">
              <Form.Item name="ai_enabled" label="启用 AI 识别" valuePropName="checked">
                <Switch />
              </Form.Item>
              <Form.Item
                name="ai_api_key"
                label="API Key"
                extra="支持 OpenAI 兼容接口（默认 PinCC）。Key 仅存本地数据库，不会上传。"
              >
                <Input.Password placeholder="sk-..." readOnly={!aiEnabled} />
              </Form.Item>
              <Form.Item name="ai_api_base" label="API 地址">
                <Input placeholder="https://v2.pincc.ai/v1" readOnly={!aiEnabled} />
              </Form.Item>
              <Form.Item name="ai_model" label="模型">
                <Input placeholder="gpt-4o-mini" readOnly={!aiEnabled} />
              </Form.Item>
            </Card>
          </Col>

          <Col xs={24} lg={12}>
            <Card className="stat-card" title="行情设置">
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
          </Col>
        </Row>
      </Form>

      <div className="settings-page__footer">
        <Button type="primary" loading={loading} onClick={handleSave}>
          保存设置
        </Button>
      </div>

      <Card className="stat-card" title="数据管理" style={{ marginTop: 8 }}>
        <Typography.Paragraph type="secondary" copyable={dbPath ? { text: dbPath } : undefined}>
          数据库路径：{dbPath || "加载中..."}
        </Typography.Paragraph>
        <Typography.Paragraph type="secondary">
          建议定期导出备份。导入会覆盖当前全部数据，请谨慎操作。
        </Typography.Paragraph>
        <Button icon={<DownloadOutlined />} onClick={handleExport} style={{ marginRight: 8 }}>
          导出备份
        </Button>
        <Upload accept=".json" showUploadList={false} beforeUpload={beforeImport}>
          <Button icon={<UploadOutlined />}>导入恢复</Button>
        </Upload>
      </Card>

      <Modal
        title="确认导入数据"
        open={importModalOpen}
        onCancel={() => {
          setImportModalOpen(false);
          setPendingImportFile(null);
        }}
        onOk={handleImportConfirm}
        okText="确认导入"
        okButtonProps={{ danger: true }}
      >
        <Typography.Paragraph>
          导入将<strong>覆盖当前全部数据</strong>，此操作不可撤销。建议先导出当前备份。
        </Typography.Paragraph>
        {pendingImportFile && (
          <Typography.Text type="secondary">文件：{pendingImportFile.name}</Typography.Text>
        )}
      </Modal>
    </div>
  );
}
