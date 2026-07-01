import { theme as antTheme } from "antd";
import type { ThemeConfig } from "antd";
import type { ResolvedTheme } from "./types";

const DARK = {
  bg: "#0c0e12",
  sider: "#12151b",
  surface: "#171a21",
  surfaceElevated: "#1d212b",
  border: "rgba(255, 255, 255, 0.09)",
  text: "#eceef2",
  textSecondary: "#9aa3af",
  primary: "#6aabff",
  menuActive: "rgba(106, 171, 255, 0.14)",
  menuHover: "rgba(255, 255, 255, 0.05)",
};

export function getAntdThemeConfig(resolved: ResolvedTheme): ThemeConfig {
  const isDark = resolved === "dark";

  return {
    algorithm: isDark ? antTheme.darkAlgorithm : antTheme.defaultAlgorithm,
    token: {
      colorPrimary: isDark ? DARK.primary : "#1677ff",
      borderRadius: 8,
      fontFamily:
        '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "PingFang SC", "Microsoft YaHei", sans-serif',
      ...(isDark
        ? {
            colorBgLayout: DARK.bg,
            colorBgContainer: DARK.surface,
            colorBgElevated: DARK.surfaceElevated,
            colorBorder: DARK.border,
            colorBorderSecondary: "rgba(255, 255, 255, 0.06)",
            colorText: DARK.text,
            colorTextSecondary: DARK.textSecondary,
            colorFillAlter: DARK.surfaceElevated,
            colorFillSecondary: "rgba(255, 255, 255, 0.06)",
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
            headerBg: DARK.surface,
            siderBg: DARK.sider,
            bodyBg: DARK.bg,
            triggerBg: DARK.sider,
          }
        : {
            headerBg: "#ffffff",
            siderBg: "#ffffff",
            bodyBg: "#f4f6fa",
          },
      Menu: isDark
        ? {
            itemBorderRadius: 8,
            itemMarginInline: 8,
            itemHeight: 40,
            itemBg: "transparent",
            itemColor: DARK.textSecondary,
            itemHoverBg: DARK.menuHover,
            itemHoverColor: DARK.text,
            itemSelectedBg: DARK.menuActive,
            itemSelectedColor: DARK.primary,
            activeBarBorderWidth: 0,
          }
        : {
            itemBorderRadius: 8,
            itemMarginInline: 8,
            itemHeight: 40,
          },
      Card: {
        borderRadiusLG: 12,
        ...(isDark
          ? {
              colorBgContainer: DARK.surface,
              colorBorderSecondary: "rgba(255, 255, 255, 0.06)",
            }
          : {}),
      },
      Table: isDark
        ? {
            headerBg: "#141820",
            borderColor: "rgba(255, 255, 255, 0.06)",
          }
        : {
            headerBg: "#fafbfc",
          },
      Button: {
        borderRadius: 8,
      },
      Segmented: isDark
        ? {
            itemSelectedBg: DARK.surfaceElevated,
            trackBg: DARK.sider,
          }
        : {},
    },
  };
}
