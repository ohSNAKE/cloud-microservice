import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import type { QuantGeneratedSignal } from "../types";

export async function notifyGeneratedQuantSignals(generatedSignals: QuantGeneratedSignal[]) {
  const eligibleSignals = generatedSignals.filter(
    (item) => item.desktop_notification_enabled,
  );

  if (eligibleSignals.length === 0) {
    return { sent: 0, denied: false };
  }

  let granted = await isPermissionGranted();

  if (!granted) {
    const permission = await requestPermission();
    granted = permission === "granted";
  }

  if (!granted) {
    return { sent: 0, denied: true };
  }

  for (const { signal } of eligibleSignals) {
    const directionLabel =
      signal.direction === "buy_attention" ? "触发买入关注" : "触发卖出关注";
    sendNotification({
      title: `量化提醒：${signal.name || signal.code} ${directionLabel}`,
      body: "当前价格进入自动网格触发区，仅供参考。",
    });
  }

  return { sent: eligibleSignals.length, denied: false };
}
