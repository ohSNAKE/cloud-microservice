import { theme as antTheme } from "antd";
import type { ThemeConfig } from "antd";
import type { ResolvedTheme } from "./types";

export function getAntdThemeConfig(resolved: ResolvedTheme): ThemeConfig {
  const isDark = resolved === "dark";

  return {
    algorithm: isDark ? antTheme.darkAlgorithm : antTheme.defaultAlgorithm,
    token: {
      colorPrimary: "#1677ff",
      borderRadius: 8,
      fontFamily:
        '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "PingFang SC", "Microsoft YaHei", sans-serif',
      ...(isDark
        ? {
            colorBgLayout: "#0f1419",
            colorText: "#e8eaed",
            colorTextSecondary: "#9aa0a6",
          }
        : {
            colorBgLayout: "#f4f6fa",
            colorText: "#1d2939",
            colorTextSecondary: "#667085",
          }),
    },
    components: {
      Layout: isDark
        ? {
            headerBg: "#1a1f26",
            siderBg: "#1a1f26",
            bodyBg: "#0f1419",
          }
        : {
            headerBg: "#ffffff",
            siderBg: "#ffffff",
            bodyBg: "#f4f6fa",
          },
      Menu: {
        itemBorderRadius: 8,
        itemMarginInline: 8,
        itemHeight: 40,
      },
      Card: {
        borderRadiusLG: 12,
      },
      Table: isDark
        ? {
            headerBg: "#141922",
          }
        : {
            headerBg: "#fafbfc",
          },
      Button: {
        borderRadius: 8,
      },
    },
  };
}
