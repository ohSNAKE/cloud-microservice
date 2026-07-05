import { useCallback, useState } from "react";
import { Input } from "antd";
import { LockOutlined } from "@ant-design/icons";
import { api } from "../api";

interface LockScreenProps {
  onUnlocked: () => void;
}

export default function LockScreen({ onUnlocked }: LockScreenProps) {
  const [password, setPassword] = useState("");
  const [verifying, setVerifying] = useState(false);
  const [hasError, setHasError] = useState(false);

  const tryUnlock = useCallback(async () => {
    const value = password.trim();
    if (!value || verifying) return;

    setVerifying(true);
    setHasError(false);
    try {
      await api.verifyAppLock(value);
      onUnlocked();
    } catch {
      setHasError(true);
      setPassword("");
    } finally {
      setVerifying(false);
    }
  }, [onUnlocked, password, verifying]);

  return (
    <div className="app-lock-screen">
      <div className="app-lock-screen__backdrop" aria-hidden />
      <div className="app-lock-screen__glass" aria-hidden />
      <div className="app-lock-screen__content">
        <Input.Password
          value={password}
          prefix={<LockOutlined />}
          placeholder="密码"
          size="large"
          autoFocus
          disabled={verifying}
          status={hasError ? "error" : undefined}
          className={`app-lock-screen__input${hasError ? " app-lock-screen__input--shake" : ""}`}
          aria-label="密码"
          onChange={(e) => {
            setPassword(e.target.value);
            if (hasError) setHasError(false);
          }}
          onPressEnter={() => void tryUnlock()}
        />
      </div>
    </div>
  );
}
