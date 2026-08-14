use crate::db::get_setting;
use crate::models::{Account, AiConfig, Category, ParsedTransactionBatch};
use crate::services::ai_parser::parse_transactions_nl;
use crate::AppState;
use tauri::State;

fn map_category(row: &rusqlite::Row) -> rusqlite::Result<Category> {
    Ok(Category {
        id: row.get(0)?,
        name: row.get(1)?,
        r#type: row.get(2)?,
        icon: row.get(3)?,
    })
}

fn map_account(row: &rusqlite::Row) -> rusqlite::Result<Account> {
    Ok(Account {
        id: row.get(0)?,
        name: row.get(1)?,
        r#type: row.get(2)?,
        balance: row.get(3)?,
        created_at: row.get(4)?,
    })
}

fn load_ai_config(conn: &rusqlite::Connection) -> AiConfig {
    AiConfig {
        ai_enabled: get_setting(conn, "ai_enabled")
            .map(|v| v == "true")
            .unwrap_or(false),
        ai_api_key: get_setting(conn, "ai_api_key").unwrap_or_default(),
        ai_api_base: get_setting(conn, "ai_api_base")
            .unwrap_or_else(|| "https://v2.pincc.ai/v1".to_string()),
        ai_model: get_setting(conn, "ai_model").unwrap_or_else(|| "gpt-4o-mini".to_string()),
    }
}

#[derive(Debug, serde::Deserialize)]
struct AiModelsResponse {
    data: Vec<AiModelItem>,
}

#[derive(Debug, serde::Deserialize)]
struct AiModelItem {
    id: String,
}

fn parse_ai_model_ids(body: &str) -> Result<Vec<String>, String> {
    let parsed: AiModelsResponse =
        serde_json::from_str(body).map_err(|e| format!("模型列表响应解析失败: {e}"))?;
    let mut ids = parsed
        .data
        .into_iter()
        .map(|model| model.id.trim().to_string())
        .filter(|id| !id.is_empty())
        .collect::<Vec<_>>();
    ids.sort();
    ids.dedup();
    Ok(ids)
}

fn validate_ai_model_request(api_base: &str, api_key: &str) -> Result<(String, String), String> {
    let base = api_base.trim().trim_end_matches('/').to_string();
    let key = api_key.trim().to_string();
    if base.is_empty() {
        return Err("请先填写 API 地址".to_string());
    }
    if key.is_empty() {
        return Err("请先填写 API Key".to_string());
    }
    Ok((base, key))
}

fn map_ai_models_response(
    result: Result<ureq::Response, ureq::Error>,
) -> Result<ureq::Response, String> {
    match result {
        Ok(response) => Ok(response),
        Err(ureq::Error::Status(status, response)) => {
            let err_body = response.into_string().unwrap_or_default();
            Err(format!("模型列表接口错误 ({status}): {err_body}"))
        }
        Err(e) => Err(format!("模型列表请求失败: {e}")),
    }
}

#[tauri::command]
pub async fn list_ai_models_command(
    api_base: String,
    api_key: String,
) -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let (base, key) = validate_ai_model_request(&api_base, &api_key)?;
        let url = format!("{base}/models");
        let response = map_ai_models_response(
            ureq::AgentBuilder::new()
                .timeout_connect(std::time::Duration::from_secs(15))
                .timeout_read(std::time::Duration::from_secs(45))
                .build()
                .get(&url)
                .set("Authorization", &format!("Bearer {key}"))
                .call(),
        )?;

        let body = response
            .into_string()
            .map_err(|e| format!("模型列表响应读取失败: {e}"))?;
        parse_ai_model_ids(&body)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn parse_transaction_nl_command(
    state: State<'_, AppState>,
    text: String,
) -> Result<ParsedTransactionBatch, String> {
    let (categories, accounts, config) = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;

        let categories = {
            let mut stmt = conn
                .prepare("SELECT id, name, type, icon FROM categories ORDER BY id")
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], map_category)
                .map_err(|e| e.to_string())?;
            rows.filter_map(Result::ok).collect::<Vec<_>>()
        };

        let accounts = {
            let mut stmt = conn
                .prepare("SELECT id, name, type, balance, created_at FROM accounts ORDER BY id")
                .map_err(|e| e.to_string())?;
            let rows = stmt.query_map([], map_account).map_err(|e| e.to_string())?;
            rows.filter_map(Result::ok).collect::<Vec<_>>()
        };

        let config = load_ai_config(&conn);
        (categories, accounts, config)
    };

    let input = text.trim().to_string();

    tauri::async_runtime::spawn_blocking(move || {
        parse_transactions_nl(&input, &categories, &accounts, &config)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ai_models_keeps_non_empty_ids_sorted() {
        let json = r#"{"data":[{"id":"gpt-4o"},{"id":""},{"id":"gpt-4o-mini"},{"id":"gpt-4o"}]}"#;
        assert_eq!(
            parse_ai_model_ids(json).unwrap(),
            vec!["gpt-4o", "gpt-4o-mini"]
        );
    }

    #[test]
    fn parse_ai_models_allows_empty_data() {
        assert_eq!(
            parse_ai_model_ids(r#"{"data":[]}"#).unwrap(),
            Vec::<String>::new()
        );
    }

    #[test]
    fn parse_ai_models_rejects_malformed_json() {
        assert!(parse_ai_model_ids("not json").is_err());
    }

    #[test]
    fn parse_ai_models_status_error_preserves_body() {
        let response = ureq::Response::new(401, "Unauthorized", r#"{"error":"bad key"}"#).unwrap();
        let err = map_ai_models_response(Err(ureq::Error::Status(401, response))).unwrap_err();
        assert_eq!(err, r#"模型列表接口错误 (401): {"error":"bad key"}"#);
    }

    #[test]
    fn validate_ai_model_request_requires_base_and_key() {
        assert!(validate_ai_model_request("", "sk-test").is_err());
        assert!(validate_ai_model_request("https://example.com/v1", "").is_err());
        assert_eq!(
            validate_ai_model_request("https://example.com/v1/", "sk-test").unwrap(),
            ("https://example.com/v1".to_string(), "sk-test".to_string())
        );
    }
}
