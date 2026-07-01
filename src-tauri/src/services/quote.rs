use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct EastMoneyResponse {
    data: Option<EastMoneyData>,
}

#[derive(Debug, Deserialize)]
struct EastMoneyData {
    f43: Option<f64>,
    f58: Option<String>,
}

fn stock_secid(code: &str) -> String {
    if code.starts_with('6') {
        format!("1.{code}")
    } else {
        format!("0.{code}")
    }
}

fn http_get(url: &str) -> Result<String, String> {
    ureq::get(url)
        .set("User-Agent", "Mozilla/5.0")
        .set("Referer", "https://finance.eastmoney.com/")
        .call()
        .map_err(|e| e.to_string())?
        .into_string()
        .map_err(|e| e.to_string())
}

pub async fn fetch_stock_price(code: &str) -> Result<f64, String> {
    let secid = stock_secid(code);
    let url = format!(
        "https://push2.eastmoney.com/api/qt/stock/get?secid={secid}&fields=f43,f58"
    );

    let text = tokio::task::spawn_blocking(move || http_get(&url))
        .await
        .map_err(|e| e.to_string())??;

    let body: EastMoneyResponse = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let price_cents = body
        .data
        .and_then(|d| d.f43)
        .ok_or_else(|| format!("无法获取股票 {code} 行情"))?;

    Ok(price_cents / 100.0)
}

pub async fn fetch_fund_price(code: &str) -> Result<f64, String> {
    let url = format!("https://fundgz.1234567.com.cn/js/{code}.js");

    let text = tokio::task::spawn_blocking(move || http_get(&url))
        .await
        .map_err(|e| e.to_string())??;

    let json_part = text
        .trim_start_matches("jsonpgz(")
        .trim_end_matches(");")
        .trim_end_matches(')');

    #[derive(Deserialize)]
    struct FundQuote {
        gsz: Option<String>,
        dwjz: Option<String>,
    }

    let quote: FundQuote = serde_json::from_str(json_part).map_err(|e| e.to_string())?;
    let price_str = quote
        .gsz
        .or(quote.dwjz)
        .ok_or_else(|| format!("无法获取基金 {code} 行情"))?;

    price_str.parse::<f64>().map_err(|e| e.to_string())
}

pub async fn lookup_stock_name(code: &str) -> Result<String, String> {
    let secid = stock_secid(code);
    let url = format!(
        "https://push2.eastmoney.com/api/qt/stock/get?secid={secid}&fields=f58"
    );

    let text = tokio::task::spawn_blocking(move || http_get(&url))
        .await
        .map_err(|e| e.to_string())??;

    let body: EastMoneyResponse = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    body.data
        .and_then(|d| d.f58)
        .ok_or_else(|| "无法获取名称".to_string())
}
