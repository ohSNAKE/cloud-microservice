use crate::services::screener::{is_central_soe, CentralControllerRegistry};
use crate::services::screener_source::{ControllerClassification, ScreenerUniverseRecord};
use std::collections::{BTreeMap, HashMap};

pub const EASTMONEY_LIST_URL: &str = "https://82.push2.eastmoney.com/api/qt/clist/get";
pub const EASTMONEY_DATACENTER_URL: &str = "https://datacenter-web.eastmoney.com/api/data/v1/get";

#[derive(Debug, Clone, PartialEq)]
pub struct EastmoneyUniversePage {
    pub total_pages: usize,
    pub page_no: usize,
    pub rows: Vec<EastmoneyUniverseRow>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EastmoneyUniverseRow {
    pub code: String,
    pub name: String,
    pub market_cap_cny: f64,
    pub market: String,
    pub eastmoney_secid: String,
}

pub fn collect_exchange_pages(
    exchange: &str,
    pages: Vec<EastmoneyUniversePage>,
) -> Result<Vec<ScreenerUniverseRecord>, String> {
    if pages.is_empty() {
        return Err("missing exchange pages".into());
    }

    let total_pages = pages[0].total_pages;
    if total_pages == 0 || pages.iter().any(|page| page.total_pages != total_pages) {
        return Err("inconsistent page totals".into());
    }

    let mut by_page = BTreeMap::new();
    for page in pages {
        if page.page_no == 0 || page.page_no > total_pages || by_page.insert(page.page_no, page).is_some() {
            return Err("duplicate or out-of-range page".into());
        }
    }
    for page_no in 1..=total_pages {
        if !by_page.contains_key(&page_no) {
            return Err("missing declared page".into());
        }
    }

    let mut by_identity: HashMap<(String, String), ScreenerUniverseRecord> = HashMap::new();
    for page in by_page.into_values() {
        for row in page.rows {
            let record = normalize_row(exchange, row)?;
            if !record.ordinary_equity {
                continue;
            }
            let key = (record.code.clone(), record.exchange.clone());
            if let Some(existing) = by_identity.get(&key) {
                if existing.eastmoney_secid != record.eastmoney_secid {
                    return Err("conflicting duplicate security identifier".into());
                }
                continue;
            }
            by_identity.insert(key, record);
        }
    }

    let mut records = by_identity.into_values().collect::<Vec<_>>();
    records.sort_by(|left, right| left.code.cmp(&right.code).then_with(|| left.exchange.cmp(&right.exchange)));
    if records.is_empty() {
        return Err("exchange has no ordinary equity records".into());
    }
    Ok(records)
}

pub fn validate_complete_universe(
    sh: Vec<ScreenerUniverseRecord>,
    sz: Vec<ScreenerUniverseRecord>,
    bj: Vec<ScreenerUniverseRecord>,
) -> Result<Vec<ScreenerUniverseRecord>, String> {
    validate_exchange_records("sh", &sh)?;
    validate_exchange_records("sz", &sz)?;
    validate_exchange_records("bj", &bj)?;

    let mut records = Vec::with_capacity(sh.len() + sz.len() + bj.len());
    records.extend(sh);
    records.extend(sz);
    records.extend(bj);
    Ok(records)
}

pub fn classify_controller(
    actual_controller: &str,
    registry: &CentralControllerRegistry,
) -> ControllerClassification {
    let trimmed = actual_controller.trim();
    if trimmed.is_empty() {
        return ControllerClassification::Unknown;
    }
    if is_central_soe(trimmed, registry) {
        return ControllerClassification::CentralSoe;
    }
    if matches!(trimmed, "地方国有企业" | "地方国资委" | "地方国企") || trimmed.contains("国资委") {
        return ControllerClassification::LocalSoe;
    }
    ControllerClassification::NonSoe
}

fn normalize_row(exchange: &str, row: EastmoneyUniverseRow) -> Result<ScreenerUniverseRecord, String> {
    if row.code.trim().is_empty() || row.name.trim().is_empty() {
        return Err("missing universe identity".into());
    }
    if !row.market_cap_cny.is_finite() || row.market_cap_cny < 0.0 {
        return Err("invalid market capitalization".into());
    }
    let expected_secid = expected_secid(exchange, &row.code)?;
    if row.eastmoney_secid != expected_secid {
        return Err("unexpected security identifier".into());
    }

    Ok(ScreenerUniverseRecord {
        code: row.code,
        exchange: exchange.into(),
        name: row.name,
        eastmoney_secid: row.eastmoney_secid,
        ordinary_equity: is_ordinary_equity(exchange, &expected_secid[2..]),
        market_cap_cny: row.market_cap_cny,
        actual_controller: String::new(),
        controller_classification: ControllerClassification::Unknown,
    })
}

fn validate_exchange_records(exchange: &str, records: &[ScreenerUniverseRecord]) -> Result<(), String> {
    if records.is_empty() {
        return Err(format!("{exchange} universe is empty"));
    }
    if records.iter().any(|record| record.exchange != exchange || !record.ordinary_equity) {
        return Err(format!("{exchange} universe contains invalid records"));
    }
    Ok(())
}

fn expected_secid(exchange: &str, code: &str) -> Result<String, String> {
    match exchange {
        "sh" => Ok(format!("1.{code}")),
        "sz" | "bj" => Ok(format!("0.{code}")),
        _ => Err("unsupported exchange".into()),
    }
}

fn is_ordinary_equity(exchange: &str, code: &str) -> bool {
    match exchange {
        "sh" => ["600", "601", "603", "605", "688"].iter().any(|prefix| code.starts_with(prefix)),
        "sz" => ["000", "001", "002", "003", "300", "301"].iter().any(|prefix| code.starts_with(prefix)),
        "bj" => ["4", "8", "9"].iter().any(|prefix| code.starts_with(prefix)),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::screener::{CentralControllerEntry, CentralControllerRegistry};
    use crate::services::screener_source::{ControllerClassification, ScreenerUniverseRecord};

    fn registry() -> CentralControllerRegistry {
        CentralControllerRegistry::from_entries(vec![CentralControllerEntry::new(
            "中国移动通信集团有限公司",
            vec!["中国移动".into()],
        )])
    }

    fn page(total_pages: usize, page_no: usize, rows: Vec<EastmoneyUniverseRow>) -> EastmoneyUniversePage {
        EastmoneyUniversePage {
            total_pages,
            page_no,
            rows,
        }
    }

    fn sh_row(code: &str) -> Vec<EastmoneyUniverseRow> {
        vec![sh_row_with_secid(code, &format!("1.{code}"))]
    }

    fn sz_row(code: &str) -> ScreenerUniverseRecord {
        universe_record(code, "sz")
    }

    fn sh_universe_row(code: &str) -> ScreenerUniverseRecord {
        universe_record(code, "sh")
    }

    fn sh_row_with_secid(code: &str, secid: &str) -> EastmoneyUniverseRow {
        EastmoneyUniverseRow {
            code: code.into(),
            name: "测试".into(),
            market_cap_cny: 50_000_000_000.0,
            market: "sh".into(),
            eastmoney_secid: secid.into(),
        }
    }

    fn universe_record(code: &str, exchange: &str) -> ScreenerUniverseRecord {
        ScreenerUniverseRecord {
            code: code.into(),
            exchange: exchange.into(),
            name: "测试".into(),
            eastmoney_secid: format!("1.{code}"),
            ordinary_equity: true,
            market_cap_cny: 50_000_000_000.0,
            actual_controller: "中国移动通信集团有限公司".into(),
            controller_classification: ControllerClassification::CentralSoe,
        }
    }

    #[test]
    fn all_declared_pages_are_required_and_totals_must_stay_consistent() {
        assert!(collect_exchange_pages("sh", vec![page(2, 1, sh_row("600001"))]).is_err());
        assert!(collect_exchange_pages(
            "sh",
            vec![
                page(2, 1, sh_row("600001")),
                page(3, 2, sh_row("600002")),
            ],
        )
        .is_err());
    }

    #[test]
    fn equal_duplicates_are_accepted_but_conflicting_identifiers_are_rejected() {
        assert_eq!(
            collect_exchange_pages(
                "sh",
                vec![page(
                    1,
                    1,
                    vec![sh_row_with_secid("600001", "1.600001"), sh_row_with_secid("600001", "1.600001")],
                )],
            )
            .unwrap()
            .len(),
            1,
        );
        assert!(collect_exchange_pages(
            "sh",
            vec![
                page(1, 1, vec![sh_row_with_secid("600001", "1.600001")]),
                page(1, 1, vec![sh_row_with_secid("600001", "1.999999")]),
            ],
        )
        .is_err());
    }

    #[test]
    fn universe_requires_ordinary_equity_in_each_exchange_and_exact_controller_classification() {
        assert!(validate_complete_universe(
            vec![sh_universe_row("600001")],
            vec![sz_row("000001")],
            vec![],
        )
        .is_err());
        assert_eq!(
            classify_controller("中国移动通信集团有限公司", &registry()),
            ControllerClassification::CentralSoe,
        );
        assert_eq!(
            classify_controller("某市国资委", &registry()),
            ControllerClassification::LocalSoe,
        );
        assert_eq!(
            classify_controller("民营企业", &registry()),
            ControllerClassification::NonSoe,
        );
        assert_eq!(classify_controller("", &registry()), ControllerClassification::Unknown);
    }
}
