import type { KlineBar, QuantTimeBucket } from "../types";

export interface IntradaySeries {
  bars: KlineBar[];
  previousClose: number | null;
}

export interface IntradayAxisBounds {
  priceMin: number;
  priceMax: number;
  percentMin: number;
  percentMax: number;
}

export interface IntradayWindowArea {
  name: "卖T关注" | "买回关注";
  start: string;
  end: string;
  labelOffset: number;
  color: string;
}

const providerTimestamp = /^\d{4}-(?:0[1-9]|1[0-2])-(?:0[1-9]|[12]\d|3[01]) (?:[01]\d|2[0-3]):[0-5]\d(?::[0-5]\d)?$/;
const sessionLabels = new Set(["09:30", "11:30", "13:00", "15:00"]);

export function selectIntradaySeries(bars: KlineBar[]): IntradaySeries {
  const validBars = bars
    .filter((bar) => providerTimestamp.test(bar.date))
    .sort((left, right) => (left.date < right.date ? -1 : left.date > right.date ? 1 : 0));

  const latestDate = validBars[validBars.length - 1]?.date.slice(0, 10);
  if (!latestDate) return { bars: [], previousClose: null };

  const previousDates = validBars
    .map((bar) => bar.date.slice(0, 10))
    .filter((date) => date < latestDate);
  const previousDate = previousDates[previousDates.length - 1];
  const previousCloses = previousDate
    ? validBars
        .filter((bar) => bar.date.slice(0, 10) === previousDate)
        .map((bar) => bar.close)
        .filter((close) => Number.isFinite(close) && close > 0)
    : [];
  const previousClose = previousDate
    ? previousCloses[previousCloses.length - 1] ?? null
    : null;

  return {
    bars: validBars.filter((bar) => bar.date.slice(0, 10) === latestDate),
    previousClose,
  };
}

export function getIntradayAxisBounds(
  bars: KlineBar[],
  previousClose: number | null,
): IntradayAxisBounds | null {
  if (previousClose === null) return null;

  const d = bars.reduce(
    (maximum, bar) =>
      Math.max(
        maximum,
        Number.isFinite(bar.low) ? Math.abs(bar.low - previousClose) : 0,
        Number.isFinite(bar.high) ? Math.abs(bar.high - previousClose) : 0,
      ),
    previousClose * 0.01,
  );
  const percent = (d / previousClose) * 100;

  return {
    priceMin: previousClose - d,
    priceMax: previousClose + d,
    percentMin: -percent,
    percentMax: percent,
  };
}

export function shouldShowSessionLabel(time: string): boolean {
  return sessionLabels.has(time);
}

function windowArea(
  times: string[],
  window: QuantTimeBucket,
  name: IntradayWindowArea["name"],
  color: string,
): IntradayWindowArea | null {
  const [start, end] = window.split("-");
  const matchingTimes = times.filter((time) => time >= start && time <= end);
  if (matchingTimes.length === 0) return null;

  return {
    name,
    start: matchingTimes[0],
    end: matchingTimes[matchingTimes.length - 1],
    labelOffset: 0,
    color,
  };
}

export function buildIntradayWindowAreas(
  times: string[],
  sellWindows: QuantTimeBucket[],
  buybackWindows: QuantTimeBucket[],
): IntradayWindowArea[] {
  const sellAreas = sellWindows
    .map((window) => windowArea(times, window, "卖T关注", "rgba(239, 83, 80, 0.12)"))
    .filter((area): area is IntradayWindowArea => area !== null);
  const buybackAreas = buybackWindows
    .map((window) => windowArea(times, window, "买回关注", "rgba(38, 166, 154, 0.12)"))
    .filter((area): area is IntradayWindowArea => area !== null)
    .map((area) => ({
      ...area,
      labelOffset: sellAreas.some((sell) => sell.start <= area.end && area.start <= sell.end) ? 16 : 0,
    }));

  return [...sellAreas, ...buybackAreas];
}
