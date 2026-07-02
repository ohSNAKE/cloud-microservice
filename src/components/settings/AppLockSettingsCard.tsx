import { useCallback, useEffect, useState } from "react";
import {
  Button,
  Card,
  Form,
  Input,
  Modal,
  Switch,
  Typography,
  message,
} from "antd";
import { LockOutlined } from "@ant-design/icons";
import { api, formatInvokeError } from "../../api";
import type { AppLockStatus } from "../../types";

export default function AppLockSettingsCard() {
  const [status, setStatus] = useState<AppLockStatus>({ enabled: false, configured: false });
  const [loading, setLoading] = useState(true);
  const [setupOpen, setSetupOpen] = useState(false);
  const [changeOpen, setChangeOpen] = useState(false);
  const [disableOpen, setDisableOpen] = useState(false);
  const [setupForm] = Form.useForm<{ password: string; confirm: string }>();
  const [changeForm] = Form.useForm<{ old_password: string; new_password: string; confirm: string }>();
  const [disableForm] = Form.useForm<{ password: string }>();

  const loadStatus = useCallback(async () => {
    setLoading(true);
    try {
      setStatus(await api.getAppLockStatus());
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadStatus();
  }, [loadStatus]);

  const handleToggle = (checked: boolean) => {
    if (checked) {
      setupForm.resetFields();
      setSetupOpen(true);
      return;
    }
    disableForm.resetFields();
    setDisableOpen(true);
  };

  const handleSetup = async () => {
    const values = await setupForm.validateFields();
    if (values.password !== values.confirm) {
      message.error("两次输入的密码不一致");
      return;
    }
    try {
      await api.setupAppLock(values.password);
      message.success("应用锁已开启，下次启动需输入密码");
      setSetupOpen(false);
      await loadStatus();
    } catch (e) {
      message.error(formatInvokeError(e));
    }
  };

  const handleChangePassword = async () => {
    const values = await changeForm.validateFields();
    if (values.new_password !== values.confirm) {
      message.error("两次输入的新密码不一致");
      return;
    }
    try {
      await api.changeAppLockPassword(values.old_password, values.new_password);
      message.success("密码已修改");
      setChangeOpen(false);
    } catch (e) {
      message.error(formatInvokeError(e));
    }
  };

  const handleDisable = async () => {
    const values = await disableForm.validateFields();
    try {
      await api.disableAppLock(values.password);
      message.success("应用锁已关闭");
      setDisableOpen(false);
      await loadStatus();
    } catch (e) {
      message.error(formatInvokeError(e));
    }
  };

  return (
    <>
      <Card className="stat-card" title="应用锁" loading={loading}>
        <Typography.Paragraph type="secondary" style={{ marginBottom: 16 }}>
          开启后，每次打开应用需输入密码才能查看财务数据；也可点击顶栏锁图标随时锁定。密码仅保存在本机，不会上传。
        </Typography.Paragraph>
        <Form layout="vertical">
          <Form.Item label="开启应用锁">
            <Switch checked={status.enabled && status.configured} onChange={handleToggle} />
          </Form.Item>
        </Form>
        {status.configured && status.enabled && (
          <Button icon={<LockOutlined />} onClick={() => { changeForm.resetFields(); setChangeOpen(true); }}>
            修改密码
          </Button>
        )}
      </Card>

      <Modal
        title="设置应用锁密码"
        open={setupOpen}
        onCancel={() => setSetupOpen(false)}
        onOk={handleSetup}
        okText="开启"
        destroyOnClose
      >
        <Typography.Paragraph type="secondary" style={{ marginBottom: 16 }}>
          请设置至少 4 位密码，忘记密码将无法恢复数据访问（需自行备份后重置）。
        </Typography.Paragraph>
        <Form form={setupForm} layout="vertical">
          <Form.Item name="password" label="密码" rules={[{ required: true, min: 4, message: "至少 4 位" }]}>
            <Input.Password placeholder="输入密码" />
          </Form.Item>
          <Form.Item
            name="confirm"
            label="确认密码"
            rules={[{ required: true, message: "请再次输入密码" }]}
          >
            <Input.Password placeholder="再次输入密码" />
          </Form.Item>
        </Form>
      </Modal>

      <Modal
        title="修改密码"
        open={changeOpen}
        onCancel={() => setChangeOpen(false)}
        onOk={handleChangePassword}
        okText="保存"
        destroyOnClose
      >
        <Form form={changeForm} layout="vertical">
          <Form.Item name="old_password" label="当前密码" rules={[{ required: true }]}>
            <Input.Password />
          </Form.Item>
          <Form.Item name="new_password" label="新密码" rules={[{ required: true, min: 4, message: "至少 4 位" }]}>
            <Input.Password />
          </Form.Item>
          <Form.Item name="confirm" label="确认新密码" rules={[{ required: true }]}>
            <Input.Password />
          </Form.Item>
        </Form>
      </Modal>

      <Modal
        title="关闭应用锁"
        open={disableOpen}
        onCancel={() => setDisableOpen(false)}
        onOk={handleDisable}
        okText="确认关闭"
        okButtonProps={{ danger: true }}
        destroyOnClose
      >
        <Typography.Paragraph type="secondary" style={{ marginBottom: 16 }}>
          输入当前密码以关闭应用锁
        </Typography.Paragraph>
        <Form form={disableForm} layout="vertical">
          <Form.Item name="password" label="当前密码" rules={[{ required: true }]}>
            <Input.Password />
          </Form.Item>
        </Form>
      </Modal>
    </>
  );
}
