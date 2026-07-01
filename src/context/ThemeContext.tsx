import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { api } from "../api";
import { getChartColors, type ChartColors } from "../constants/chartTheme";
import { applyThemeToDocument, getSystemTheme, resolveTheme } from "../theme/resolve";
import {
  THEME_STORAGE_KEY,
  isThemeMode,
  type ResolvedTheme,
  type ThemeMode,
} from "../theme/types";

interface ThemeContextValue {
  mode: ThemeMode;
  resolved: ResolvedTheme;
  chartColors: ChartColors;
  setMode: (mode: ThemeMode) => void;
}

const ThemeContext = createContext<ThemeContextValue | null>(null);

function readStoredMode(): ThemeMode {
  try {
    const stored = localStorage.getItem(THEME_STORAGE_KEY);
    if (isThemeMode(stored)) return stored;
  } catch {
    /* ignore */
  }
  return "system";
}

function persistMode(mode: ThemeMode) {
  try {
    localStorage.setItem(THEME_STORAGE_KEY, mode);
  } catch {
    /* ignore */
  }
}

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [mode, setModeState] = useState<ThemeMode>(readStoredMode);
  const [resolved, setResolved] = useState<ResolvedTheme>(() => resolveTheme(readStoredMode()));

  const applyMode = useCallback((nextMode: ThemeMode) => {
    const nextResolved = resolveTheme(nextMode);
    setModeState(nextMode);
    setResolved(nextResolved);
    applyThemeToDocument(nextResolved);
    persistMode(nextMode);
  }, []);

  const setMode = useCallback(
    (nextMode: ThemeMode) => {
      applyMode(nextMode);
      void (async () => {
        try {
          const current = await api.getSettings();
          if (current.theme_mode === nextMode) return;
          await api.updateSettings({ ...current, theme_mode: nextMode });
        } catch {
          /* 离线或未就绪时仅保留 localStorage */
        }
      })();
    },
    [applyMode],
  );

  useEffect(() => {
    applyThemeToDocument(resolved);
  }, [resolved]);

  useEffect(() => {
    if (mode !== "system") return;
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const onChange = () => setResolved(getSystemTheme());
    media.addEventListener("change", onChange);
    return () => media.removeEventListener("change", onChange);
  }, [mode]);

  useEffect(() => {
    void (async () => {
      try {
        const settings = await api.getSettings();
        if (isThemeMode(settings.theme_mode) && settings.theme_mode !== mode) {
          applyMode(settings.theme_mode);
        }
      } catch {
        /* 使用 localStorage 初始值 */
      }
    })();
    // 仅在挂载时与数据库同步
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const chartColors = useMemo(() => getChartColors(resolved), [resolved]);

  const value = useMemo(
    () => ({ mode, resolved, chartColors, setMode }),
    [mode, resolved, chartColors, setMode],
  );

  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>;
}

export function useTheme() {
  const ctx = useContext(ThemeContext);
  if (!ctx) throw new Error("useTheme must be used within ThemeProvider");
  return ctx;
}
