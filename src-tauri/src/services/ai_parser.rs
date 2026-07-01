use crate::db::today;
use crate::models::AiConfig;
use crate::models::{Account, Category, ParsedTransactionDraft};
use chrono::{Duration, Local, NaiveDate};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Debug, Deserialize)]
struct ChatMessage {
    content: String,
}

#[derive(Debug, Deserialize)]
struct LlmParseResult {
    r#type: Option<String>,
    amount: Option<f64>,
    category_name: Option<String>,
    account_name: Option<String>,
    transaction_date: Option<String>,
    note: Option<String>,
    confidence: Option<f64>,
}

pub fn parse_transaction_nl(
    text: &str,
    categories: &[Category],
    accounts: &[Account],
    config: &AiConfig,
) -> Result<ParsedTransactionDraft, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("请输入记账内容".to_string());
    }

    let draft = if config.ai_enabled && !config.ai_api_key.trim().is_empty() {
        match parse_with_llm(trimmed, categories, accounts, config) {
            Ok(mut draft) => {
                draft.parse_notice = None;
                draft
            }
            Err(err) => {
                eprintln!("AI 解析失败，使用规则兜底: {err}");
                let mut draft = parse_with_rules(trimmed, categories, accounts)?;
                let brief = if err.contains("503") {
                    "503 服务暂时不可用".to_string()
                } else if err.len() > 60 {
                    format!("{}…", &err[..60])
                } else {
                    err.clone()
                };
                draft.parse_notice =
                    Some(format!("AI 暂不可用（{brief}），已使用本地规则识别"));
                draft
            }
        }
    } else {
        parse_with_rules(trimmed, categories, accounts)?
    };

    Ok(enrich_draft(trimmed, draft, categories, accounts))
}

fn parse_with_llm(
    text: &str,
    categories: &[Category],
    accounts: &[Account],
    config: &AiConfig,
) -> Result<ParsedTransactionDraft, String> {
    let category_lines: Vec<String> = categories
        .iter()
        .map(|c| format!("- {} ({})", c.name, c.r#type))
        .collect();
    let account_lines: Vec<String> = accounts
        .iter()
        .map(|a| format!("- {} ({})", a.name, a.r#type))
        .collect();

    let today_str = today();
    let system_prompt = format!(
        "你是个人财务记账助手。根据用户的中文自然语言描述，提取一条记账记录。\n\
         只返回 JSON，不要 markdown，字段如下：\n\
         {{\"type\":\"expense|income\",\"amount\":数字,\"category_name\":\"分类名\",\"account_name\":\"账户名\",\"transaction_date\":\"YYYY-MM-DD\",\"note\":\"备注\",\"confidence\":0到1}}\n\
         规则：\n\
         1. type 只能是 expense 或 income\n\
         2. amount 必须是正数\n\
         3. category_name 和 account_name 必须从下列列表中选择最接近的一项\n\
         4. 未说明日期时使用今天 {today_str}\n\
         5. note 保留用户描述中的关键信息\n\
         可用分类：\n{}\n\
         可用账户：\n{}",
        category_lines.join("\n"),
        account_lines.join("\n")
    );

    let base = config.ai_api_base.trim().trim_end_matches('/');
    let url = format!("{base}/chat/completions");
    let body = serde_json::json!({
        "model": config.ai_model,
        "temperature": 0.1,
        "response_format": { "type": "json_object" },
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": text }
        ]
    });

    let response = ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(15))
        .timeout_read(std::time::Duration::from_secs(90))
        .build()
        .post(&url)
        .set("Authorization", &format!("Bearer {}", config.ai_api_key.trim()))
        .set("Content-Type", "application/json")
        .send_json(body)
        .map_err(|e| format!("AI 请求失败: {e}"))?;

    if !(200..300).contains(&response.status()) {
        let status = response.status();
        let err_body = response.into_string().unwrap_or_default();
        return Err(format!("AI 接口错误 ({status}): {err_body}"));
    }

    let parsed: ChatCompletionResponse = response
        .into_json()
        .map_err(|e| format!("AI 响应解析失败: {e}"))?;

    let content = parsed
        .choices
        .first()
        .map(|c| c.message.content.trim())
        .filter(|c| !c.is_empty())
        .ok_or_else(|| "AI 未返回有效内容".to_string())?;

    let llm: LlmParseResult = serde_json::from_str(content)
        .map_err(|e| format!("AI JSON 解析失败: {e}, 原始内容: {content}"))?;

    let tx_type = normalize_type(llm.r#type.as_deref().unwrap_or("expense"));
    let amount = llm.amount.ok_or_else(|| "AI 未识别到金额".to_string())?;
    if amount <= 0.0 {
        return Err("AI 识别的金额无效".to_string());
    }

    Ok(ParsedTransactionDraft {
        r#type: tx_type.to_string(),
        amount,
        category_id: None,
        category_name: llm.category_name,
        account_id: None,
        account_name: llm.account_name,
        transaction_date: llm
            .transaction_date
            .filter(|d| parse_date(d).is_some())
            .unwrap_or_else(today),
        note: llm.note.unwrap_or_else(|| text.to_string()),
        confidence: llm.confidence.unwrap_or(0.85).clamp(0.0, 1.0),
        source: "ai".to_string(),
        raw_text: text.to_string(),
        parse_notice: None,
    })
}

fn parse_with_rules(
    text: &str,
    _categories: &[Category],
    accounts: &[Account],
) -> Result<ParsedTransactionDraft, String> {
    let tx_type = detect_type(text);
    let amount = extract_amount(text).ok_or_else(|| "未能识别金额，请包含具体数字".to_string())?;
    let transaction_date = extract_date(text).unwrap_or_else(today);
    let category_name = detect_category_name(text, tx_type);
    let account_name = detect_account_name(text, accounts);

    Ok(ParsedTransactionDraft {
        r#type: tx_type.to_string(),
        amount,
        category_id: None,
        category_name,
        account_id: None,
        account_name,
        transaction_date,
        note: text.to_string(),
        confidence: 0.55,
        source: "rule".to_string(),
        raw_text: text.to_string(),
        parse_notice: None,
    })
}

fn enrich_draft(
    text: &str,
    mut draft: ParsedTransactionDraft,
    categories: &[Category],
    accounts: &[Account],
) -> ParsedTransactionDraft {
    let tx_type = normalize_type(&draft.r#type);
    draft.r#type = tx_type.to_string();

    if draft.category_id.is_none() {
        draft.category_id = resolve_category_id(
            draft.category_name.as_deref(),
            text,
            tx_type,
            categories,
        );
        if draft.category_name.is_none() {
            draft.category_name = draft
                .category_id
                .and_then(|id| categories.iter().find(|c| c.id == id).map(|c| c.name.clone()));
        }
    }

    if draft.account_id.is_none() {
        draft.account_id =
            resolve_account_id(draft.account_name.as_deref(), text, accounts);
        if draft.account_name.is_none() {
            draft.account_name = draft
                .account_id
                .and_then(|id| accounts.iter().find(|a| a.id == id).map(|a| a.name.clone()));
        }
    }

    if draft.note.trim().is_empty() {
        draft.note = text.to_string();
    }

    draft
}

fn normalize_type(raw: &str) -> &'static str {
    match raw.trim().to_lowercase().as_str() {
        "income" | "收入" => "income",
        _ => "expense",
    }
}

fn detect_type(text: &str) -> &'static str {
    let income_keywords = ["工资", "收入", "收到", "入账", "奖金", "到账", "发薪", "退款"];
    if income_keywords.iter().any(|k| text.contains(k)) {
        "income"
    } else {
        "expense"
    }
}

fn extract_amount(text: &str) -> Option<f64> {
    let keywords = [
        "花了", "花费", "消费", "支出", "付了", "支付", "买了", "用了", "收入", "收到", "入账",
        "转入", "转出",
    ];
    for keyword in keywords {
        if let Some(idx) = text.find(keyword) {
            let tail = &text[idx + keyword.len()..];
            if let Some(amount) = first_amount(tail) {
                return Some(amount);
            }
        }
    }

    for suffix in ["块钱", "块", "元"] {
        if let Some(idx) = text.find(suffix) {
            let head = &text[..idx];
            if let Some(amount) = last_amount(head) {
                return Some(amount);
            }
        }
    }

    last_amount(text)
}

fn first_amount(text: &str) -> Option<f64> {
    if let Some(n) = scan_ascii_numbers(text).into_iter().next() {
        return Some(n);
    }
    parse_chinese_amount_in_text(text)
}

fn last_amount(text: &str) -> Option<f64> {
    if let Some(n) = scan_ascii_numbers(text).into_iter().last() {
        return Some(n);
    }
    parse_chinese_amount_in_text(text)
}

fn parse_chinese_amount_in_text(text: &str) -> Option<f64> {
    for suffix in ["块钱", "块", "元"] {
        if let Some(idx) = text.find(suffix) {
            let before = text[..idx].trim();
            let cn: String = before
                .chars()
                .rev()
                .take_while(|c| is_chinese_num_char(*c))
                .collect::<String>()
                .chars()
                .rev()
                .collect();
            if let Some(v) = chinese_to_number(&cn) {
                return Some(v);
            }
        }
    }
    None
}

fn is_chinese_num_char(c: char) -> bool {
    matches!(
        c,
        '零' | '〇'
            | '一' | '二' | '两' | '三' | '四' | '五' | '六' | '七' | '八' | '九'
            | '壹' | '贰' | '叁' | '肆' | '伍' | '陆' | '柒' | '捌' | '玖'
            | '十' | '拾' | '百' | '佰' | '千' | '仟' | '万' | '萬'
    )
}

fn chinese_to_number(text: &str) -> Option<f64> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }

    let digit = |c: char| -> Option<f64> {
        match c {
            '零' | '〇' => Some(0.0),
            '一' | '壹' => Some(1.0),
            '二' | '两' | '贰' => Some(2.0),
            '三' | '叁' => Some(3.0),
            '四' | '肆' => Some(4.0),
            '五' | '伍' => Some(5.0),
            '六' | '陆' => Some(6.0),
            '七' | '柒' => Some(7.0),
            '八' | '捌' => Some(8.0),
            '九' | '玖' => Some(9.0),
            _ => None,
        }
    };

    let mut total = 0.0;
    let mut section = 0.0;
    let mut number = 0.0;

    for c in text.chars() {
        if let Some(d) = digit(c) {
            number = d;
            continue;
        }
        match c {
            '十' | '拾' => {
                if number == 0.0 {
                    number = 1.0;
                }
                section += number * 10.0;
                number = 0.0;
            }
            '百' | '佰' => {
                section += if number == 0.0 { 0.0 } else { number * 100.0 };
                number = 0.0;
            }
            '千' | '仟' => {
                section += number * 1000.0;
                number = 0.0;
            }
            '万' | '萬' => {
                section += number;
                total += section * 10000.0;
                section = 0.0;
                number = 0.0;
            }
            _ => return None,
        }
    }

    let val = total + section + number;
    if val > 0.0 { Some(val) } else { None }
}

fn scan_ascii_numbers(text: &str) -> Vec<f64> {
    let mut numbers = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c.is_ascii_digit() {
            let mut num = String::new();
            num.push(c);
            while chars
                .peek()
                .map(|next| next.is_ascii_digit() || *next == '.')
                .unwrap_or(false)
            {
                num.push(chars.next().unwrap());
            }
            if let Ok(value) = num.parse::<f64>() {
                if value > 0.0 {
                    numbers.push(value);
                }
            }
        }
    }
    numbers
}

fn extract_date(text: &str) -> Option<String> {
    if text.contains("今天") || text.contains("今日") {
        return Some(today());
    }
    if text.contains("昨天") || text.contains("昨日") {
        return Local::now()
            .date_naive()
            .checked_sub_signed(Duration::days(1))
            .map(|d| d.format("%Y-%m-%d").to_string());
    }
    if text.contains("前天") {
        return Local::now()
            .date_naive()
            .checked_sub_signed(Duration::days(2))
            .map(|d| d.format("%Y-%m-%d").to_string());
    }

    if let Some(date) = find_iso_date(text) {
        return Some(date);
    }

    None
}

fn find_iso_date(text: &str) -> Option<String> {
    for (byte_idx, ch) in text.char_indices() {
        if !ch.is_ascii_digit() {
            continue;
        }
        if text.len().saturating_sub(byte_idx) < 10 {
            break;
        }
        let candidate = &text[byte_idx..byte_idx + 10];
        if candidate.as_bytes().get(4) == Some(&b'-')
            && candidate.as_bytes().get(7) == Some(&b'-')
            && parse_date(candidate).is_some()
        {
            return Some(candidate.to_string());
        }
    }
    None
}

fn parse_date(value: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()
}

fn detect_category_name(text: &str, tx_type: &str) -> Option<String> {
    let rules: HashMap<&str, &str> = [
        ("餐饮", "餐饮|午饭|午餐|晚饭|晚餐|早餐|吃饭|外卖|咖啡|奶茶|火锅|烧烤|包子|馒头|饺子"),
        ("交通", "交通|地铁|公交|打车|滴滴|出租|高铁|火车|机票|加油|停车"),
        ("购物", "购物|淘宝|京东|拼多多|买|超市|商场|衣服|鞋"),
        ("住房", "住房|房租|租金|物业|水电|燃气|房贷"),
        ("娱乐", "娱乐|电影|游戏|KTV|旅游|旅行|门票"),
        ("医疗", "医疗|医院|药|看病|体检"),
        ("教育", "教育|书|课程|培训|学费"),
        ("工资", "工资|薪水|发薪"),
        ("奖金", "奖金|年终奖|红包"),
        ("理财收益", "理财|基金收益|股息|分红|利息"),
    ]
    .into_iter()
    .collect();

    for (category, pattern) in rules {
        for keyword in pattern.split('|') {
            if text.contains(keyword) {
                return Some(category.to_string());
            }
        }
    }

    if tx_type == "income" {
        Some("其他收入".to_string())
    } else {
        Some("其他支出".to_string())
    }
}

fn detect_account_name(text: &str, accounts: &[Account]) -> Option<String> {
    let keyword_map = [
        ("微信", "wechat"),
        ("支付宝", "alipay"),
        ("银行卡", "bank"),
        ("银行", "bank"),
        ("现金", "cash"),
        ("证券", "broker"),
    ];

    for (keyword, acc_type) in keyword_map {
        if text.contains(keyword) {
            if let Some(acc) = accounts.iter().find(|a| a.r#type == acc_type) {
                return Some(acc.name.clone());
            }
        }
    }

    accounts.first().map(|a| a.name.clone())
}

fn resolve_category_id(
    name: Option<&str>,
    text: &str,
    tx_type: &str,
    categories: &[Category],
) -> Option<i64> {
    let typed: Vec<&Category> = categories
        .iter()
        .filter(|c| c.r#type == tx_type)
        .collect();

    if let Some(name) = name {
        if let Some(cat) = typed.iter().find(|c| c.name == name) {
            return Some(cat.id);
        }
        if let Some(cat) = typed.iter().find(|c| name.contains(&c.name) || c.name.contains(name)) {
            return Some(cat.id);
        }
    }

    let guessed = detect_category_name(text, tx_type)?;
    typed
        .iter()
        .find(|c| c.name == guessed)
        .map(|c| c.id)
        .or_else(|| typed.first().map(|c| c.id))
}

fn resolve_account_id(name: Option<&str>, text: &str, accounts: &[Account]) -> Option<i64> {
    if let Some(name) = name {
        if let Some(acc) = accounts.iter().find(|a| a.name == name) {
            return Some(acc.id);
        }
        if let Some(acc) = accounts
            .iter()
            .find(|a| name.contains(&a.name) || a.name.contains(name))
        {
            return Some(acc.id);
        }
    }

    let guessed = detect_account_name(text, accounts)?;
    accounts
        .iter()
        .find(|a| a.name == guessed)
        .map(|a| a.id)
        .or_else(|| accounts.first().map(|a| a.id))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_categories() -> Vec<Category> {
        vec![
            Category {
                id: 1,
                name: "餐饮".to_string(),
                r#type: "expense".to_string(),
                icon: "🍜".to_string(),
            },
            Category {
                id: 2,
                name: "交通".to_string(),
                r#type: "expense".to_string(),
                icon: "🚗".to_string(),
            },
            Category {
                id: 3,
                name: "工资".to_string(),
                r#type: "income".to_string(),
                icon: "💰".to_string(),
            },
        ]
    }

    fn sample_accounts() -> Vec<Account> {
        vec![
            Account {
                id: 1,
                name: "微信".to_string(),
                r#type: "wechat".to_string(),
                balance: 0.0,
                created_at: String::new(),
            },
            Account {
                id: 2,
                name: "支付宝".to_string(),
                r#type: "alipay".to_string(),
                balance: 0.0,
                created_at: String::new(),
            },
        ]
    }

    #[test]
    fn rule_parser_extracts_expense() {
        let config = AiConfig::default();
        let draft = parse_transaction_nl(
            "今天中午用微信花了35块买午餐",
            &sample_categories(),
            &sample_accounts(),
            &config,
        )
        .expect("parse");

        assert_eq!(draft.r#type, "expense");
        assert!((draft.amount - 35.0).abs() < f64::EPSILON);
        assert_eq!(draft.category_id, Some(1));
        assert_eq!(draft.account_id, Some(1));
        assert_eq!(draft.source, "rule");
    }

    #[test]
    fn rule_parser_detects_income() {
        let config = AiConfig::default();
        let draft = parse_transaction_nl(
            "收到工资8000元",
            &sample_categories(),
            &sample_accounts(),
            &config,
        )
        .expect("parse");

        assert_eq!(draft.r#type, "income");
        assert!((draft.amount - 8000.0).abs() < f64::EPSILON);
        assert_eq!(draft.category_id, Some(3));
    }

    #[test]
    fn rule_parser_chinese_numeral_amount() {
        let config = AiConfig::default();
        let draft = parse_transaction_nl(
            "微信花了两块钱买包子",
            &sample_categories(),
            &sample_accounts(),
            &config,
        )
        .expect("parse");

        assert!((draft.amount - 2.0).abs() < f64::EPSILON);
        assert_eq!(draft.account_id, Some(1));
        assert_eq!(draft.category_id, Some(1));
    }
}
