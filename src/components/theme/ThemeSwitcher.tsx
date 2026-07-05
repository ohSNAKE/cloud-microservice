import { DesktopOutlined, MoonOutlined, SunOutlined } from "@ant-design/icons";
import { Segmented } from "antd";
import type { ReactNode } from "react";
import { useTheme } from "../../context/ThemeContext";
import { THEME_MODE_OPTIONS, type ThemeMode } from "../../theme/types";

const ICONS: Record<ThemeMode, ReactNode> = {
  light: <SunOutlined />,
  dark: <MoonOutlined />,
  system: <DesktopOutlined />,
};

interface ThemeSwitcherProps {
  size?: "small" | "middle" | "large";
  block?: boolean;
}

export default function ThemeSwitcher({ size = "middle", block }: ThemeSwitcherProps) {
  const { mode, setMode } = useTheme();
  const compact = size === "small";

  return (
    <Segmented
      block={block}
      size={size}
      value={mode}
      onChange={(value) => setMode(value as ThemeMode)}
      options={THEME_MODE_OPTIONS.map((opt) => ({
        label: compact ? (
          <span title={opt.label} aria-label={opt.label}>
            {ICONS[opt.value]}
          </span>
        ) : (
          <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
            {ICONS[opt.value]}
            {opt.label}
          </span>
        ),
        value: opt.value,
      }))}
    />
  );
}
