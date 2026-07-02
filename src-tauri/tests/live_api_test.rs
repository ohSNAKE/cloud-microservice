//! 行情接口联调测试（需要网络）

#[tokio::test]
async fn live_fetch_stock_price() {
    let price = finance_assistant_lib::services::quote::fetch_stock_price("600519")
        .await
        .expect("fetch stock price");
    assert!(price > 0.0, "stock price should be positive, got {price}");
    println!("600519 price: {price}");
}

#[tokio::test]
async fn live_fetch_fund_price() {
    let price = finance_assistant_lib::services::quote::fetch_fund_price("000001")
        .await
        .expect("fetch fund price");
    assert!(price > 0.0, "fund price should be positive, got {price}");
    println!("000001 fund nav: {price}");
}

#[tokio::test]
async fn live_fetch_stock_kline() {
    let bars = finance_assistant_lib::services::quote::fetch_stock_kline("600519", "day", 30)
        .await
        .expect("fetch stock kline");
    assert!(!bars.is_empty(), "kline should not be empty");
    assert!(bars[0].close > 0.0);
    println!("kline bars: {}, latest: {}", bars.len(), bars.last().unwrap().date);
}

#[tokio::test]
async fn live_fetch_fund_kline() {
    let bars = finance_assistant_lib::services::quote::fetch_fund_kline("000001", 30)
        .await
        .expect("fetch fund history");
    assert!(!bars.is_empty(), "fund history should not be empty");
    println!("fund bars: {}, latest nav: {}", bars.len(), bars.last().unwrap().close);
}

#[tokio::test]
async fn live_lookup_stock_name() {
    let name = finance_assistant_lib::services::quote::lookup_stock_name("600519")
        .await
        .expect("lookup name");
    assert!(!name.is_empty());
    println!("600519 name: {name}");
}

#[tokio::test]
async fn live_lookup_fund_name() {
    let name = finance_assistant_lib::services::quote::lookup_fund_name("513180")
        .await
        .expect("lookup fund name");
    assert!(!name.is_empty());
    assert!(name.contains("恒生"));
    println!("513180 name: {name}");
}
