use crate::services::screener::{normalize_controller_name, CentralControllerEntry};
use chrono::NaiveDate;
use scraper::{Html, Selector};
use serde::Deserialize;
use std::collections::HashSet;

pub const SASAC_DIRECTORY_URL: &str =
    "https://www.sasac.gov.cn/n2588035/n2641579/n2641645/index.html";
pub const EMBEDDED_ALIAS_REGISTRY: &str =
    include_str!("../../resources/sasac_controller_aliases.json");

#[derive(Debug, Clone)]
pub struct SasacDirectorySnapshot {
    pub source_url: String,
    pub source_date: String,
    pub entries: Vec<CentralControllerEntry>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SasacAliasEntry {
    pub legal_name: String,
    pub normalized_legal_name: String,
    pub alias: String,
    pub normalized_alias: String,
    pub source_url: String,
    pub announcement_date: String,
}

#[derive(Debug, Deserialize)]
struct RawAliasEntry {
    legal_name: String,
    alias: String,
    source_url: String,
    announcement_date: String,
}

pub fn parse_sasac_directory(
    source_date: &str,
    html: &str,
) -> Result<SasacDirectorySnapshot, String> {
    NaiveDate::parse_from_str(source_date, "%Y-%m-%d").map_err(|error| error.to_string())?;
    let document = Html::parse_document(html);
    let selector = Selector::parse("article a").map_err(|error| error.to_string())?;
    let mut seen = HashSet::new();
    let mut entries = Vec::new();

    for node in document.select(&selector) {
        let legal_name = node.text().collect::<String>().trim().to_string();
        if legal_name.is_empty() {
            continue;
        }
        let normalized_name = normalize_controller_name(&legal_name);
        if !seen.insert(normalized_name.clone()) {
            return Err("duplicate SASAC legal name".into());
        }
        entries.push(CentralControllerEntry {
            original_name: legal_name,
            normalized_name,
            aliases: Vec::new(),
        });
    }

    if entries.is_empty() {
        return Err("empty SASAC directory".into());
    }

    Ok(SasacDirectorySnapshot {
        source_url: SASAC_DIRECTORY_URL.into(),
        source_date: source_date.into(),
        entries,
    })
}

pub fn parse_alias_registry(json: &str) -> Result<Vec<SasacAliasEntry>, String> {
    let raw_entries =
        serde_json::from_str::<Vec<RawAliasEntry>>(json).map_err(|error| error.to_string())?;
    raw_entries
        .into_iter()
        .map(|entry| {
            if entry.legal_name.trim().is_empty() || entry.alias.trim().is_empty() {
                return Err("alias registry entry is missing identity".into());
            }
            if !entry.source_url.starts_with("https://www.sasac.gov.cn/") {
                return Err("alias registry source must be an official SASAC URL".into());
            }
            NaiveDate::parse_from_str(&entry.announcement_date, "%Y-%m-%d")
                .map_err(|error| error.to_string())?;

            Ok(SasacAliasEntry {
                normalized_legal_name: normalize_controller_name(&entry.legal_name),
                normalized_alias: normalize_controller_name(&entry.alias),
                legal_name: entry.legal_name,
                alias: entry.alias,
                source_url: entry.source_url,
                announcement_date: entry.announcement_date,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sasac_directory_preserves_source_metadata_and_aliases() {
        let html = r#"<article><a>中国移动通信集团有限公司</a></article>"#;
        let snapshot = parse_sasac_directory("2026-07-21", html).unwrap();
        assert_eq!(snapshot.source_url, SASAC_DIRECTORY_URL);
        assert!(snapshot
            .entries
            .iter()
            .any(|entry| entry.normalized_name == "中国移动通信"));
    }

    #[test]
    fn versioned_alias_registry_requires_official_provenance() {
        let aliases = parse_alias_registry(
            r#"[{"legal_name":"中国移动通信集团有限公司","alias":"中国移动","source_url":"https://www.sasac.gov.cn/example","announcement_date":"2020-01-01"}]"#,
        )
        .unwrap();
        assert_eq!(aliases[0].alias, "中国移动");
        assert!(parse_alias_registry(
            r#"[{"legal_name":"中国移动通信集团有限公司","alias":"中国移动"}]"#,
        )
        .is_err());
    }

    #[test]
    fn malformed_or_empty_directory_is_not_a_valid_registry() {
        assert!(parse_sasac_directory("2026-07-21", "<html></html>").is_err());
    }
}
