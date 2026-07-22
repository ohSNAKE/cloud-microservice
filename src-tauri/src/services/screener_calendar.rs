use crate::models::ScreenerRefreshSchedule;
use chrono::{DateTime, Datelike, Duration, FixedOffset, NaiveDate, NaiveTime, SecondsFormat, Timelike};
use serde::Deserialize;
use std::collections::HashSet;

const HOLIDAYS_JSON: &str = include_str!("../../resources/china_market_holidays.json");

#[derive(Debug, Deserialize)]
struct HolidayRegistry {
    closed_dates: Vec<String>,
}

pub fn schedule_at(now: DateTime<FixedOffset>) -> ScreenerRefreshSchedule {
    let china_now = now.with_timezone(&FixedOffset::east_opt(8 * 3600).unwrap());
    let holidays = holiday_dates();
    let next = next_refresh_china_time(china_now, &holidays);
    ScreenerRefreshSchedule {
        should_refresh_now: is_live_session(china_now, &holidays),
        next_refresh_at: next
            .with_timezone(&chrono::Utc)
            .to_rfc3339_opts(SecondsFormat::Secs, true),
    }
}

fn next_refresh_china_time(
    now: DateTime<FixedOffset>,
    holidays: &HashSet<NaiveDate>,
) -> DateTime<FixedOffset> {
    let morning_open = NaiveTime::from_hms_opt(9, 30, 0).unwrap();
    let morning_close = NaiveTime::from_hms_opt(11, 30, 0).unwrap();
    let afternoon_open = NaiveTime::from_hms_opt(13, 0, 0).unwrap();
    let afternoon_close = NaiveTime::from_hms_opt(15, 0, 0).unwrap();
    let date = now.date_naive();
    let time = now.time();

    if !is_trading_day(date, holidays) {
        return at_china_time(next_trading_day(date, holidays), morning_open);
    }
    if time < morning_open {
        return at_china_time(date, morning_open);
    }
    if (morning_open..=morning_close).contains(&time) || (afternoon_open..=afternoon_close).contains(&time) {
        let aligned = align_next_quarter(now + Duration::minutes(1));
        if aligned.time() <= morning_close || ((afternoon_open..=afternoon_close).contains(&aligned.time())) {
            return aligned;
        }
    }
    if time > morning_close && time < afternoon_open {
        return at_china_time(date, afternoon_open);
    }
    at_china_time(next_trading_day(date + Duration::days(1), holidays), morning_open)
}

fn is_live_session(now: DateTime<FixedOffset>, holidays: &HashSet<NaiveDate>) -> bool {
    if !is_trading_day(now.date_naive(), holidays) {
        return false;
    }
    let time = now.time();
    (NaiveTime::from_hms_opt(9, 30, 0).unwrap()..=NaiveTime::from_hms_opt(11, 30, 0).unwrap()).contains(&time)
        || (NaiveTime::from_hms_opt(13, 0, 0).unwrap()..=NaiveTime::from_hms_opt(15, 0, 0).unwrap()).contains(&time)
}

fn align_next_quarter(time: DateTime<FixedOffset>) -> DateTime<FixedOffset> {
    let minute = time.minute();
    let add_minutes = (15 - minute % 15) % 15;
    let add_minutes = if add_minutes == 0 { 15 } else { add_minutes };
    let aligned = time + Duration::minutes(add_minutes as i64);
    aligned
        .with_second(0)
        .and_then(|value| value.with_nanosecond(0))
        .unwrap()
}

fn next_trading_day(mut date: NaiveDate, holidays: &HashSet<NaiveDate>) -> NaiveDate {
    while !is_trading_day(date, holidays) {
        date += Duration::days(1);
    }
    date
}

fn is_trading_day(date: NaiveDate, holidays: &HashSet<NaiveDate>) -> bool {
    !matches!(date.weekday(), chrono::Weekday::Sat | chrono::Weekday::Sun) && !holidays.contains(&date)
}

fn at_china_time(date: NaiveDate, time: NaiveTime) -> DateTime<FixedOffset> {
    date.and_time(time)
        .and_local_timezone(FixedOffset::east_opt(8 * 3600).unwrap())
        .single()
        .unwrap()
}

fn holiday_dates() -> HashSet<NaiveDate> {
    serde_json::from_str::<HolidayRegistry>(HOLIDAYS_JSON)
        .map(|registry| {
            registry
                .closed_dates
                .into_iter()
                .filter_map(|date| NaiveDate::parse_from_str(&date, "%Y-%m-%d").ok())
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{FixedOffset, TimeZone};

    fn china_time(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> chrono::DateTime<FixedOffset> {
        FixedOffset::east_opt(8 * 3600)
            .unwrap()
            .with_ymd_and_hms(year, month, day, hour, minute, 0)
            .single()
            .unwrap()
    }

    #[test]
    fn schedule_returns_next_opening_during_break_weekend_holiday_and_after_close() {
        assert_eq!(schedule_at(china_time(2026, 7, 21, 11, 31)).next_refresh_at, "2026-07-21T05:00:00Z");
        assert_eq!(schedule_at(china_time(2026, 7, 25, 10, 0)).next_refresh_at, "2026-07-27T01:30:00Z");
        assert_eq!(schedule_at(china_time(2026, 10, 1, 10, 0)).next_refresh_at, "2026-10-02T01:30:00Z");
    }

    #[test]
    fn schedule_marks_live_session_and_aligns_to_fifteen_minutes() {
        let schedule = schedule_at(china_time(2026, 7, 21, 9, 31));
        assert!(schedule.should_refresh_now);
        assert_eq!(schedule.next_refresh_at, "2026-07-21T01:45:00Z");
    }
}
