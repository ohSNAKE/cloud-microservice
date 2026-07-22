use crate::services::screener::{DividendAdjustment, NormalizedDividend, NormalizedKline, ScreenerPeriod};
use crate::services::screener_source::{DividendKind, NormalizedQuote, PerShareBasis, RawDividend};
use chrono::{DateTime, FixedOffset, NaiveDate, NaiveTime, SecondsFormat, TimeZone, Utc};
use std::collections::HashSet;

pub const EASTMONEY_QUOTE_URL: &str = "https://push2.eastmoney.com/api/qt/stock/get";
pub const EASTMONEY_KLINE_URL: &str = "https://push2his.eastmoney.com/api/qt/stock/kline/get";

#[derive(Debug, Clone, PartialEq)]
pub struct EastmoneyQuotePayload {
    pub code: String,
    pub price_cents: Option<f64>,
    pub observed_unix_seconds: Option<i64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EastmoneyKlineRow {
    pub date: String,
    pub close: f64,
}

pub fn parse_quote(exchange: &str, payload: EastmoneyQuotePayload) -> Result<NormalizedQuote, String> {
    let price_cents = payload.price_cents.ok_or_else(|| "missing quote price".to_string())?;
    if !price_cents.is_finite() || price_cents <= 0.0 {
        return Err("quote price must be positive and finite".into());
    }
    let observed_at = DateTime::from_timestamp(
        payload
            .observed_unix_seconds
            .ok_or_else(|| "missing quote timestamp".to_string())?,
        0,
    )
    .ok_or_else(|| "invalid quote timestamp".to_string())?
    .to_rfc3339_opts(SecondsFormat::Secs, true);

    Ok(NormalizedQuote {
        code: payload.code,
        exchange: exchange.into(),
        price: price_cents / 100.0,
        observed_at,
    })
}

pub fn parse_klines(
    code: &str,
    exchange: &str,
    period: ScreenerPeriod,
    rows: Vec<EastmoneyKlineRow>,
) -> Result<Vec<NormalizedKline>, String> {
    let china_offset = FixedOffset::east_opt(8 * 3600).unwrap();
    let close_time = NaiveTime::from_hms_opt(15, 0, 0).unwrap();
    let mut seen_completed_at = HashSet::new();
    let mut parsed = Vec::with_capacity(rows.len());

    for row in rows {
        if !row.close.is_finite() || row.close <= 0.0 {
            return Err("kline close must be positive and finite".into());
        }
        let trading_date = NaiveDate::parse_from_str(&row.date, "%Y-%m-%d")
            .map_err(|error| error.to_string())?;
        let completed_at = china_offset
            .from_local_datetime(&trading_date.and_time(close_time))
            .single()
            .ok_or_else(|| "invalid China close timestamp".to_string())?
            .with_timezone(&Utc)
            .to_rfc3339_opts(SecondsFormat::Secs, true);
        if !seen_completed_at.insert(completed_at.clone()) {
            return Err("duplicate kline timestamp".into());
        }

        parsed.push(NormalizedKline {
            code: code.into(),
            exchange: exchange.into(),
            period,
            trading_date,
            completed_at,
            close: row.close,
        });
    }

    parsed.sort_by(|left, right| left.completed_at.cmp(&right.completed_at));
    Ok(parsed)
}

pub fn normalize_dividend(
    code: &str,
    exchange: &str,
    raw: RawDividend,
    ex_date_total_capital: Option<f64>,
    valuation_date_total_capital: Option<f64>,
) -> Result<NormalizedDividend, String> {
    let ex_dividend_date = raw
        .ex_dividend_date
        .unwrap_or_else(|| NaiveDate::from_ymd_opt(1900, 1, 1).unwrap());
    let eligible_cash_kind = matches!(raw.distribution_kind, DividendKind::Cash | DividendKind::SpecialCash);
    let source_cash_per_ten_shares = raw.cash_per_ten_shares;
    let per_ex_date_share = source_cash_per_ten_shares.unwrap_or(0.0) / 10.0;
    let confirmed_cash = raw.confirmed
        && eligible_cash_kind
        && raw.ex_dividend_date.is_some()
        && per_ex_date_share.is_finite()
        && per_ex_date_share > 0.0;

    if !confirmed_cash {
        return Ok(NormalizedDividend {
            code: code.into(),
            exchange: exchange.into(),
            ex_dividend_date,
            source_cash_per_ten_shares,
            ex_date_total_capital,
            valuation_date_total_capital,
            gross_per_current_share: 0.0,
            adjustment: DividendAdjustment::Unverifiable,
            confirmed_cash: false,
        });
    }

    let (gross_per_current_share, adjustment) = match raw.per_share_basis {
        PerShareBasis::Current => (per_ex_date_share, DividendAdjustment::VerifiedUnadjusted),
        PerShareBasis::ExDate => match (ex_date_total_capital, valuation_date_total_capital) {
            (Some(ex_capital), Some(valuation_capital))
                if ex_capital.is_finite()
                    && valuation_capital.is_finite()
                    && ex_capital > 0.0
                    && valuation_capital > 0.0 =>
            {
                (
                    per_ex_date_share * ex_capital / valuation_capital,
                    DividendAdjustment::Adjusted,
                )
            }
            _ => (per_ex_date_share, DividendAdjustment::Unverifiable),
        },
        PerShareBasis::Unknown => (per_ex_date_share, DividendAdjustment::Unverifiable),
    };

    Ok(NormalizedDividend {
        code: code.into(),
        exchange: exchange.into(),
        ex_dividend_date,
        source_cash_per_ten_shares,
        ex_date_total_capital,
        valuation_date_total_capital,
        gross_per_current_share,
        adjustment,
        confirmed_cash: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::screener::{DividendAdjustment, ScreenerPeriod};
    use crate::services::screener_source::{DividendKind, PerShareBasis, RawDividend};
    use chrono::NaiveDate;

    fn naive_date(date: &str) -> NaiveDate {
        NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap()
    }

    fn invalid_price_quote() -> EastmoneyQuotePayload {
        EastmoneyQuotePayload {
            code: "600001".into(),
            price_cents: Some(0.0),
            observed_unix_seconds: Some(1_785_000_000),
        }
    }

    fn valid_quote() -> EastmoneyQuotePayload {
        EastmoneyQuotePayload {
            code: "600001".into(),
            price_cents: Some(1234.0),
            observed_unix_seconds: Some(1_785_000_000),
        }
    }

    fn invalid_timestamp_kline_payload() -> Vec<EastmoneyKlineRow> {
        vec![EastmoneyKlineRow {
            date: "invalid".into(),
            close: 10.0,
        }]
    }

    fn raw_dividend(
        cash_per_ten_shares: &str,
        ex_dividend_date: &str,
        distribution_kind: DividendKind,
        confirmed: bool,
    ) -> RawDividend {
        RawDividend {
            ex_dividend_date: Some(naive_date(ex_dividend_date)),
            cash_per_ten_shares: cash_per_ten_shares.parse::<f64>().ok(),
            distribution_kind,
            confirmed,
            per_share_basis: PerShareBasis::ExDate,
        }
    }

    fn current_basis_dividend(cash_per_ten_shares: &str) -> RawDividend {
        RawDividend {
            per_share_basis: PerShareBasis::Current,
            ..raw_dividend(cash_per_ten_shares, "2025-06-01", DividendKind::Cash, true)
        }
    }

    #[test]
    fn quote_and_kline_parser_reject_nonpositive_values_and_invalid_timestamps() {
        assert!(parse_quote("sh", invalid_price_quote()).is_err());
        assert!(parse_klines("600001", "sh", ScreenerPeriod::Week, invalid_timestamp_kline_payload()).is_err());

        let quote = parse_quote("sh", valid_quote()).unwrap();
        assert_eq!(quote.price, 12.34);
        assert!(quote.observed_at.ends_with('Z'));
    }

    #[test]
    fn cash_per_ten_and_share_capital_normalize_to_current_share_basis() {
        let record = normalize_dividend(
            "600001",
            "sh",
            raw_dividend("10", "2025-06-01", DividendKind::Cash, true),
            Some(1_000.0),
            Some(2_000.0),
        )
        .unwrap();
        assert_eq!(record.gross_per_current_share, 0.5);
        assert_eq!(record.adjustment, DividendAdjustment::Adjusted);
    }

    #[test]
    fn missing_capital_is_unverifiable_unless_provider_declares_current_share_basis() {
        assert_eq!(
            normalize_dividend(
                "600001",
                "sh",
                raw_dividend("10", "2025-06-01", DividendKind::Cash, true),
                None,
                None,
            )
            .unwrap()
            .adjustment,
            DividendAdjustment::Unverifiable,
        );
        assert_eq!(
            normalize_dividend("600001", "sh", current_basis_dividend("1.0"), None, None)
                .unwrap()
                .adjustment,
            DividendAdjustment::VerifiedUnadjusted,
        );
    }

    #[test]
    fn non_cash_unconfirmed_or_missing_date_rows_remain_noneligible_records() {
        let non_cash = normalize_dividend(
            "600001",
            "sh",
            raw_dividend("10", "2025-06-01", DividendKind::NonCash, true),
            Some(1_000.0),
            Some(1_000.0),
        )
        .unwrap();
        assert!(!non_cash.confirmed_cash);

        let unconfirmed = normalize_dividend(
            "600001",
            "sh",
            raw_dividend("10", "2025-06-01", DividendKind::SpecialCash, false),
            Some(1_000.0),
            Some(1_000.0),
        )
        .unwrap();
        assert!(!unconfirmed.confirmed_cash);

        let missing_date = normalize_dividend(
            "600001",
            "sh",
            RawDividend {
                ex_dividend_date: None,
                cash_per_ten_shares: Some(10.0),
                distribution_kind: DividendKind::Cash,
                confirmed: true,
                per_share_basis: PerShareBasis::ExDate,
            },
            Some(1_000.0),
            Some(1_000.0),
        )
        .unwrap();
        assert!(!missing_date.confirmed_cash);
    }
}
