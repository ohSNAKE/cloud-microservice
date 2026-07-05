import { useCallback, useState } from "react";
import { Input, Typography } from "antd";
import { LockOutlined } from "@ant-design/icons";
import { api } from "../api";
import AppLogo from "./AppLogo";

interface LockScreenProps {
  onUnlocked: () => void;
}

export default function LockScreen({ onUnlocked }: LockScreenProps) {
  const [password, setPassword] = useState("");
  const [verifying, setVerifying] = useState(false);
  const [error, setError] = useState("");

  const tryUnlock = useCallback(async () => {
    const value = password.trim();
    if (!value || verifying) return;

    setVerifying(true);
    setError("");
    try {
      await api.verifyAppLock(value);
      onUnlocked();
    } catch {
      setError("密码错误，请重试");
      setPassword("");
    } finally {
      setVerifying(false);
    }
  }, [onUnlocked, password, verifying]);

  return (
    <div className="app-lock-screen">
      <div className="app-lock-screen__backdrop" aria-hidden />
      <div className="app-lock-screen__glass" aria-hidden />
      <div className="app-lock-screen__panel">
        <AppLogo size={56} className="app-lock-screen__logo" />
        <Typography.Title level={4} className="app-lock-screen__title">
          应用已锁定
        </Typography.Title>
        <Typography.Text type="secondary" className="app-lock-screen__hint">
          输入密码后按回车
        </Typography.Text>

        <Input.Password
          value={password}
          prefix={<LockOutlined />}
          placeholder="密码"
          size="large"
          autoFocus
          disabled={verifying}
          className="app-lock-screen__input"
          onChange={(e) => {
            setPassword(e.target.value);
            if (error) setError("");
          }}
          onPressEnter={() => void tryUnlock()}
        />

        {error && (
          <Typography.Text type="danger" className="app-lock-screen__error">
            {error}
          </Typography.Text>
        )}
      </div>
    </div>
  );
}
