import { useState } from "react";
import { Button, Form, Input, Typography, message } from "antd";
import { LockOutlined } from "@ant-design/icons";
import { api, formatInvokeError } from "../api";

interface LockScreenProps {
  onUnlocked: () => void;
}

export default function LockScreen({ onUnlocked }: LockScreenProps) {
  const [loading, setLoading] = useState(false);
  const [form] = Form.useForm<{ password: string }>();

  const handleUnlock = async () => {
    const { password } = await form.validateFields();
    setLoading(true);
    try {
      await api.verifyAppLock(password);
      message.success("已解锁");
      onUnlocked();
    } catch (e) {
      message.error(formatInvokeError(e));
      form.setFieldsValue({ password: "" });
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="app-lock-screen">
      <div className="app-lock-screen__card stat-card">
        <div className="app-lock-screen__logo">财</div>
        <Typography.Title level={4} style={{ marginBottom: 4 }}>
          财记已锁定
        </Typography.Title>
        <Typography.Paragraph type="secondary" style={{ marginBottom: 24 }}>
          输入密码以查看您的财务数据
        </Typography.Paragraph>
        <Form form={form} layout="vertical" onFinish={handleUnlock}>
          <Form.Item name="password" rules={[{ required: true, message: "请输入密码" }]}>
            <Input.Password
              prefix={<LockOutlined />}
              placeholder="密码"
              size="large"
              autoFocus
              onPressEnter={() => form.submit()}
            />
          </Form.Item>
          <Button type="primary" htmlType="submit" block size="large" loading={loading}>
            解锁
          </Button>
        </Form>
      </div>
    </div>
  );
}
