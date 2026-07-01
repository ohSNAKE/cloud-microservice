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

fn kline_period_code(period: &str) -> i32 {
    match period {
        "week" => 102,
        "month" => 103,
        _ => 101,
    }
}

#[derive(Debug, Deserialize)]
struct StockKlineResponse {
    data: Option<StockKlineData>,
}

#[derive(Debug, Deserialize)]
struct StockKlineData {
    klines: Option<Vec<String>>,
}

pub async fn fetch_stock_kline(
    code: &str,
    period: &str,
    limit: i64,
) -> Result<Vec<crate::models::KlineBar>, String> {
    let secid = stock_secid(code);
    let klt = kline_period_code(period);
    let url = format!(
        "https://push2his.eastmoney.com/api/qt/stock/kline/get?secid={secid}&klt={klt}&fqt=1&lmt={limit}&end=20500101&fields1=f1,f2,f3,f4,f5,f6&fields2=f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61"
    );

    let text = tokio::task::spawn_blocking(move || http_get(&url))
        .await
        .map_err(|e| e.to_string())??;

    let body: StockKlineResponse = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let klines = body
        .data
        .and_then(|d| d.klines)
        .ok_or_else(|| format!("无法获取股票 {code} K线数据"))?;

    let mut bars = Vec::new();
    for line in klines {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() < 6 {
            continue;
        }
        let open = parts[1].parse::<f64>().unwrap_or(0.0);
        let close = parts[2].parse::<f64>().unwrap_or(0.0);
        let high = parts[3].parse::<f64>().unwrap_or(0.0);
        let low = parts[4].parse::<f64>().unwrap_or(0.0);
        let volume = parts[5].parse::<f64>().unwrap_or(0.0);
        let change_pct = parts.get(8).and_then(|v| v.parse().ok()).unwrap_or(0.0);
        bars.push(crate::models::KlineBar {
            date: parts[0].to_string(),
            open,
            close,
            low,
            high,
            volume,
            change_pct,
        });
    }

    if bars.is_empty() {
        return Err(format!("股票 {code} 暂无K线数据"));
    }
    Ok(bars)
}

#[derive(Debug, Deserialize)]
struct FundHistoryResponse {
    #[serde(rename = "Datas")]
    datas: Option<Vec<FundHistoryItem>>,
}

#[derive(Debug, Deserialize)]
struct FundHistoryItem {
    #[serde(rename = "FSRQ")]
    date: String,
    #[serde(rename = "DWJZ")]
    nav: String,
    #[serde(rename = "JZZZL")]
    change_pct: Option<String>,
}

pub async fn fetch_fund_kline(
    code: &str,
    limit: i64,
) -> Result<Vec<crate::models::KlineBar>, String> {
    let url = format!(
        "https://fundmobapi.eastmoney.com/FundMNewApi/FundMNHisNetList?FCODE={code}&pageIndex=1&pageSize={limit}&plat=Android&appType=ttjj&product=EFund&version=6.5.5&deviceid=1"
    );

    let text = tokio::task::spawn_blocking(move || {
        ureq::get(&url)
            .set("User-Agent", "Mozilla/5.0")
            .set("Referer", "https://fund.eastmoney.com/")
            .call()
            .map_err(|e| e.to_string())?
            .into_string()
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;

    let body: FundHistoryResponse = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let items = body
        .datas
        .ok_or_else(|| format!("无法获取基金 {code} 净值数据"))?;

    let mut bars: Vec<crate::models::KlineBar> = items
        .into_iter()
        .filter_map(|item| {
            let nav = item.nav.parse::<f64>().ok()?;
            let change_pct = item
                .change_pct
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(0.0);
            let open = if change_pct.abs() > 0.0001 {
                nav / (1.0 + change_pct / 100.0)
            } else {
                nav
            };
            Some(crate::models::KlineBar {
                date: item.date,
                open,
                close: nav,
                low: open.min(nav),
                high: open.max(nav),
                volume: 0.0,
                change_pct,
            })
        })
        .collect();

    bars.reverse();
    if bars.is_empty() {
        return Err(format!("基金 {code} 暂无净值数据"));
    }
    Ok(bars)
}
