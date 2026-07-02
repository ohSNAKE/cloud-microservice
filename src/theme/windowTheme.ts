import { getCurrentWindow } from "@tauri-apps/api/window";
import type { ResolvedTheme } from "./types";

/** macOS 顶栏与交通灯布局参数（需与 tauri.conf.json trafficLightPosition 一致） */
export const MAC_TITLEBAR = {
  trafficX: 16,
  trafficY: 20,
  clusterWidth: 52,
  gapAfterTraffic: 8,
  height: 52,
} as const;

function isTauri(): boolean {
  return typeof window !== "undefined" && ("__TAURI__" in window || "__TAURI_INTERNALS__" in window);
}

function isMac(): boolean {
  return navigator.platform.toLowerCase().includes("mac");
}

function parseColor(value: string): { red: number; green: number; blue: number; alpha: number } | null {
  const hex = value.trim();
  if (hex.startsWith("#") && (hex.length === 7 || hex.length === 4)) {
    const full =
      hex.length === 4
        ? `#${hex[1]}${hex[1]}${hex[2]}${hex[2]}${hex[3]}${hex[3]}`
        : hex;
    return {
      red: parseInt(full.slice(1, 3), 16),
      green: parseInt(full.slice(3, 5), 16),
      blue: parseInt(full.slice(5, 7), 16),
      alpha: 255,
    };
  }
  return null;
}

/** 同步原生窗口标题栏/背景与当前主题 */
export async function syncNativeWindowTheme(resolved: ResolvedTheme) {
  if (!isTauri()) return;

  try {
    const win = getCurrentWindow();
    await win.setTheme(resolved);

    const surface = getComputedStyle(document.documentElement).getPropertyValue("--color-surface").trim();
    const color = parseColor(surface);
    if (color) {
      await win.setBackgroundColor(color);
    }
  } catch {
    /* 浏览器预览或无权限时忽略 */
  }
}

export function getTitlebarInset(): number {
  return 0;
}

export function applyTitlebarInset() {
  const mac = isTauri() && isMac();
  const root = document.documentElement;

  if (mac) {
    const { trafficX, clusterWidth, gapAfterTraffic, height } = MAC_TITLEBAR;
    root.style.setProperty("--header-height", `${height}px`);
    root.style.setProperty("--header-content-inset", `${trafficX + clusterWidth + gapAfterTraffic}px`);
  } else {
    root.style.setProperty("--header-height", "56px");
    root.style.setProperty("--header-content-inset", "16px");
  }

  root.style.setProperty("--titlebar-inset", `${getTitlebarInset()}px`);
}
