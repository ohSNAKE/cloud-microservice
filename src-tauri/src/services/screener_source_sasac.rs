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

pub fn embedded_central_controller_entries() -> Vec<CentralControllerEntry> {
    [
        "中国核工业集团有限公司",
        "中国航天科技集团有限公司",
        "中国航天科工集团有限公司",
        "中国航空工业集团有限公司",
        "中国船舶集团有限公司",
        "中国兵器工业集团有限公司",
        "中国兵器装备集团有限公司",
        "中国电子科技集团有限公司",
        "中国航空发动机集团有限公司",
        "中国石油天然气集团有限公司",
        "中国石油化工集团有限公司",
        "中国海洋石油集团有限公司",
        "国家石油天然气管网集团有限公司",
        "国家电网有限公司",
        "中国南方电网有限责任公司",
        "中国华能集团有限公司",
        "中国大唐集团有限公司",
        "中国华电集团有限公司",
        "国家电力投资集团有限公司",
        "中国长江三峡集团有限公司",
        "国家能源投资集团有限责任公司",
        "中国电信集团有限公司",
        "中国联合网络通信集团有限公司",
        "中国移动通信集团有限公司",
        "中国电子信息产业集团有限公司",
        "中国第一汽车集团有限公司",
        "东风汽车集团有限公司",
        "中国一重集团有限公司",
        "中国机械工业集团有限公司",
        "哈尔滨电气集团有限公司",
        "中国东方电气集团有限公司",
        "鞍钢集团有限公司",
        "中国宝武钢铁集团有限公司",
        "中国铝业集团有限公司",
        "中国远洋海运集团有限公司",
        "中国航空集团有限公司",
        "中国东方航空集团有限公司",
        "中国南方航空集团有限公司",
        "中国中化控股有限责任公司",
        "中粮集团有限公司",
        "中国五矿集团有限公司",
        "中国通用技术集团控股有限责任公司",
        "中国建筑集团有限公司",
        "中国储备粮管理集团有限公司",
        "国家开发投资集团有限公司",
        "招商局集团有限公司",
        "华润集团有限公司",
        "中国旅游集团有限公司",
        "中国商用飞机有限责任公司",
        "中国节能环保集团有限公司",
        "中国国际工程咨询有限公司",
        "中国诚通控股集团有限公司",
        "中国中煤能源集团有限公司",
        "中国煤炭科工集团有限公司",
        "中国机械科学研究总院集团有限公司",
        "中国中钢集团有限公司",
        "中国钢研科技集团有限公司",
        "中国化学工程集团有限公司",
        "中国盐业集团有限公司",
        "中国建材集团有限公司",
        "中国有色矿业集团有限公司",
        "中国稀土集团有限公司",
        "有研科技集团有限公司",
        "矿冶科技集团有限公司",
        "中国国际技术智力合作集团有限公司",
        "中国建筑科学研究院有限公司",
        "中国中车集团有限公司",
        "中国铁路通信信号集团有限公司",
        "中国铁路工程集团有限公司",
        "中国铁道建筑集团有限公司",
        "中国交通建设集团有限公司",
        "中国信息通信科技集团有限公司",
        "中国农业发展集团有限公司",
        "中国林业集团有限公司",
        "中国医药集团有限公司",
        "中国保利集团有限公司",
        "中国建设科技有限公司",
        "中国冶金地质总局",
        "中国煤炭地质总局",
        "新兴际华集团有限公司",
        "中国民航信息集团有限公司",
        "中国航空油料集团有限公司",
        "中国航空器材集团有限公司",
        "中国电力建设集团有限公司",
        "中国能源建设集团有限公司",
        "中国安能建设集团有限公司",
        "中国黄金集团有限公司",
        "中国广核集团有限公司",
        "中国华录集团有限公司",
        "华侨城集团有限公司",
        "南光（集团）有限公司",
        "中国电气装备集团有限公司",
    ]
    .into_iter()
    .map(|name| CentralControllerEntry::new(name, Vec::new()))
    .collect()
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
