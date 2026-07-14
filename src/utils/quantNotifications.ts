import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import type { QuantDirection, QuantGeneratedSignal } from "../types";

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

  const directionLabels: Record<QuantDirection, string> = {
    buy_attention: "买入关注",
    sell_attention: "卖出关注",
    sell_t_attention: "卖T关注",
    buyback_attention: "买回关注",
  };

  for (const item of eligibleSignals) {
    const { signal } = item;
    const directionLabel = directionLabels[signal.direction] ?? signal.direction;
    const defaultBody =
      signal.source === "intraday_t"
        ? signal.direction === "sell_t_attention"
          ? `历史高点高发时段 ${signal.trigger_zone}，当前日内位置已触发阈值，可关注卖T。仅供参考。`
          : signal.direction === "buyback_attention"
            ? `历史低点高发时段 ${signal.trigger_zone}，已满足止跌确认，可关注买回。仅供参考。`
            : `历史高发时段 ${signal.trigger_zone}，当前日内位置已触发阈值，仅供参考。`
        : "当前价格进入自动网格触发区，仅供参考。";

    sendNotification({
      title: `量化提醒：${signal.name || signal.code} ${directionLabel}`,
      body: item.notification_body ?? defaultBody,
    });
  }

  return { sent: eligibleSignals.length, denied: false };
}
