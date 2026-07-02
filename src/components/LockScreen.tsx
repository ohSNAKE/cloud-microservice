import { useCallback, useState } from "react";
import { Input, Typography } from "antd";
import { LockOutlined } from "@ant-design/icons";
import { api } from "../api";

interface LockScreenProps {
  onUnlocked: () => void;
}

export default function LockScreen({ onUnlocked }: LockScreenProps) {
  const [password, setPassword] = useState("");
  const [verifying, setVerifying] = useState(false);

  const tryUnlock = useCallback(
    async (value: string, clearOnFail = false) => {
      if (!value.trim() || verifying) return;
      setVerifying(true);
      try {
        await api.verifyAppLock(value);
        onUnlocked();
      } catch {
        if (clearOnFail) {
          setPassword("");
        }
      } finally {
        setVerifying(false);
      }
    },
    [onUnlocked, verifying],
  );

  return (
    <div className="app-lock-screen">
      <div className="app-lock-screen__card stat-card">
        <div className="app-lock-screen__logo">财</div>
        <Typography.Title level={4} style={{ marginBottom: 20 }}>
          已锁定
        </Typography.Title>
        <Input.Password
          value={password}
          prefix={<LockOutlined />}
          placeholder="密码"
          size="large"
          autoFocus
          disabled={verifying}
          onChange={(e) => {
            const value = e.target.value;
            setPassword(value);
            if (value.length >= 4) {
              void tryUnlock(value, false);
            }
          }}
          onPressEnter={() => void tryUnlock(password, true)}
        />
      </div>
    </div>
  );
}
