import { ConfigProvider } from "antd";
import zhCN from "antd/locale/zh_CN";
import type { ReactNode } from "react";
import { ThemeProvider, useTheme } from "../../context/ThemeContext";
import { getAntdThemeConfig } from "../../theme/antdConfig";

function ThemedConfigProvider({ children }: { children: ReactNode }) {
  const { resolved } = useTheme();
  return (
    <ConfigProvider locale={zhCN} theme={getAntdThemeConfig(resolved)}>
      {children}
    </ConfigProvider>
  );
}

export default function AppThemeProvider({ children }: { children: ReactNode }) {
  return (
    <ThemeProvider>
      <ThemedConfigProvider>{children}</ThemedConfigProvider>
    </ThemeProvider>
  );
}
