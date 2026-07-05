import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { Spin } from "antd";
import { api } from "../api";
import LockScreen from "../components/LockScreen";
import { readManualLocked, writeManualLocked } from "../utils/appLockStorage";

interface AppLockContextValue {
  lockEnabled: boolean;
  locked: boolean;
  lock: () => void;
  unlock: () => void;
}

const AppLockContext = createContext<AppLockContextValue | null>(null);

export function useAppLock() {
  const ctx = useContext(AppLockContext);
  if (!ctx) {
    throw new Error("useAppLock must be used within AppLockProvider");
  }
  return ctx;
}

interface AppLockProviderProps {
  children: ReactNode;
}

export function AppLockProvider({ children }: AppLockProviderProps) {
  const [checking, setChecking] = useState(true);
  const [lockEnabled, setLockEnabled] = useState(false);
  const [locked, setLocked] = useState(false);

  const refreshStatus = useCallback(async () => {
    try {
      const status = await api.getAppLockStatus();
      const enabled = status.enabled && status.configured;
      setLockEnabled(enabled);
      return enabled;
    } catch {
      setLockEnabled(false);
      return false;
    }
  }, []);

  useEffect(() => {
    void (async () => {
      setChecking(true);
      const enabled = await refreshStatus();
      if (enabled && readManualLocked()) {
        setLocked(true);
      }
      setChecking(false);
    })();
  }, [refreshStatus]);

  const lock = useCallback(() => {
    if (lockEnabled) {
      writeManualLocked(true);
      setLocked(true);
    }
  }, [lockEnabled]);

  const unlock = useCallback(() => {
    writeManualLocked(false);
    setLocked(false);
  }, []);

  const value = useMemo(
    () => ({ lockEnabled, locked, lock, unlock }),
    [lockEnabled, locked, lock, unlock],
  );

  if (checking) {
    return (
      <div className="app-lock-screen app-lock-screen--loading">
        <div className="app-lock-screen__backdrop" aria-hidden />
        <div className="app-lock-screen__glass" aria-hidden />
        <div className="app-lock-screen__content">
          <Spin size="large" />
        </div>
      </div>
    );
  }

  if (locked) {
    return (
      <AppLockContext.Provider value={value}>
        <LockScreen onUnlocked={unlock} />
      </AppLockContext.Provider>
    );
  }

  return <AppLockContext.Provider value={value}>{children}</AppLockContext.Provider>;
}
