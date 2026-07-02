import { useCallback, useEffect, useState, type ReactNode } from "react";
import { Spin } from "antd";
import { api } from "../api";
import LockScreen from "./LockScreen";

interface AppLockGateProps {
  children: ReactNode;
}

export default function AppLockGate({ children }: AppLockGateProps) {
  const [checking, setChecking] = useState(true);
  const [locked, setLocked] = useState(false);

  const checkLock = useCallback(async () => {
    setChecking(true);
    try {
      const status = await api.getAppLockStatus();
      setLocked(status.enabled && status.configured);
    } catch {
      setLocked(false);
    } finally {
      setChecking(false);
    }
  }, []);

  useEffect(() => {
    void checkLock();
  }, [checkLock]);

  if (checking) {
    return (
      <div className="app-lock-screen">
        <Spin size="large" tip="加载中..." />
      </div>
    );
  }

  if (locked) {
    return <LockScreen onUnlocked={() => setLocked(false)} />;
  }

  return <>{children}</>;
}
