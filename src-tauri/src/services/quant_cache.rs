use crate::models::KlineBar;
use crate::services::quant::IntradayTWindowStats;
use std::collections::HashMap;
use std::future::Future;
use std::sync::Mutex;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct DailyHistoryKey {
    code: String,
    market: String,
    trading_date: String,
    limit: i64,
}

impl DailyHistoryKey {
    pub fn new(code: &str, market: &str, trading_date: &str, limit: i64) -> Self {
        Self {
            code: code.to_string(),
            market: market.to_string(),
            trading_date: trading_date.to_string(),
            limit,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct IntradayHistoryKey {
    code: String,
    market: String,
    trading_date: String,
    lookback_days: usize,
    high_min_count: usize,
    low_min_count: usize,
}

impl IntradayHistoryKey {
    pub fn new(
        code: &str,
        market: &str,
        trading_date: &str,
        lookback_days: usize,
        high_min_count: usize,
        low_min_count: usize,
    ) -> Self {
        Self {
            code: code.to_string(),
            market: market.to_string(),
            trading_date: trading_date.to_string(),
            lookback_days,
            high_min_count,
            low_min_count,
        }
    }
}

#[derive(Clone)]
pub struct IntradayHistoryEntry {
    pub bars: Vec<KlineBar>,
    pub stats: IntradayTWindowStats,
}

#[derive(Default)]
pub struct QuantMarketCache {
    daily_bars: HashMap<DailyHistoryKey, Vec<KlineBar>>,
    intraday: HashMap<IntradayHistoryKey, IntradayHistoryEntry>,
}

impl QuantMarketCache {
    pub fn get_daily(&self, key: &DailyHistoryKey) -> Option<Vec<KlineBar>> {
        self.daily_bars.get(key).cloned()
    }

    pub fn insert_daily(&mut self, key: DailyHistoryKey, bars: Vec<KlineBar>) {
        self.daily_bars.insert(key, bars);
    }

    pub fn get_intraday(&self, key: &IntradayHistoryKey) -> Option<IntradayHistoryEntry> {
        self.intraday.get(key).cloned()
    }

    pub fn insert_intraday(
        &mut self,
        key: IntradayHistoryKey,
        bars: Vec<KlineBar>,
        stats: IntradayTWindowStats,
    ) {
        self.intraday
            .insert(key, IntradayHistoryEntry { bars, stats });
    }
}

pub async fn get_or_fetch_daily<F, Fut>(
    cache: &Mutex<QuantMarketCache>,
    key: DailyHistoryKey,
    fetch: F,
) -> Result<Vec<KlineBar>, String>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<Vec<KlineBar>, String>>,
{
    if let Some(bars) = cache.lock().map_err(|err| err.to_string())?.get_daily(&key) {
        return Ok(bars);
    }

    let bars = fetch().await?;
    cache
        .lock()
        .map_err(|err| err.to_string())?
        .insert_daily(key, bars.clone());
    Ok(bars)
}

pub async fn get_or_fetch_intraday<F, Fut, A>(
    cache: &Mutex<QuantMarketCache>,
    key: IntradayHistoryKey,
    fetch: F,
    analyze: A,
) -> Result<IntradayHistoryEntry, String>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<Vec<KlineBar>, String>>,
    A: FnOnce(&[KlineBar]) -> IntradayTWindowStats,
{
    if let Some(entry) = cache
        .lock()
        .map_err(|err| err.to_string())?
        .get_intraday(&key)
    {
        return Ok(entry);
    }

    let bars = fetch().await?;
    let stats = analyze(&bars);
    let entry = IntradayHistoryEntry {
        bars: bars.clone(),
        stats,
    };
    cache
        .lock()
        .map_err(|err| err.to_string())?
        .insert_intraday(key, entry.bars.clone(), entry.stats.clone());
    Ok(entry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::KlineBar;
    use crate::services::quant::IntradayTWindowStats;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    fn bar(date: &str, close: f64) -> KlineBar {
        KlineBar {
            date: date.to_string(),
            open: close,
            close,
            low: close,
            high: close,
            volume: 100.0,
            change_pct: 0.0,
        }
    }

    #[test]
    fn daily_history_key_changes_on_trading_date_or_limit() {
        let mut cache = QuantMarketCache::default();
        let key = DailyHistoryKey::new("000001", "cn", "2026-07-14", 21);
        cache.insert_daily(key.clone(), vec![bar("2026-07-14", 10.0)]);

        assert!(cache.get_daily(&key).is_some());
        assert!(cache
            .get_daily(&DailyHistoryKey::new("000001", "cn", "2026-07-15", 21))
            .is_none());
        assert!(cache
            .get_daily(&DailyHistoryKey::new("000001", "cn", "2026-07-14", 22))
            .is_none());
    }

    #[test]
    fn intraday_history_key_changes_when_analysis_settings_change() {
        let mut cache = QuantMarketCache::default();
        let key = IntradayHistoryKey::new("000001", "cn", "2026-07-14", 22, 4, 4);
        cache.insert_intraday(
            key.clone(),
            vec![bar("2026-07-13 09:30", 10.0)],
            IntradayTWindowStats {
                valid_days: 10,
                high_frequency_windows: vec!["09:30-09:45".to_string()],
                low_frequency_windows: vec![],
            },
        );

        assert!(cache.get_intraday(&key).is_some());
        assert!(cache
            .get_intraday(&IntradayHistoryKey::new(
                "000001",
                "cn",
                "2026-07-14",
                23,
                4,
                4
            ))
            .is_none());
        assert!(cache
            .get_intraday(&IntradayHistoryKey::new(
                "000001",
                "cn",
                "2026-07-14",
                22,
                5,
                4
            ))
            .is_none());
    }

    #[test]
    fn cache_returns_only_successfully_inserted_history() {
        let cache = QuantMarketCache::default();
        let daily = DailyHistoryKey::new("000001", "cn", "2026-07-14", 21);
        let intraday = IntradayHistoryKey::new("000001", "cn", "2026-07-14", 22, 4, 4);

        assert!(cache.get_daily(&daily).is_none());
        assert!(cache.get_intraday(&intraday).is_none());
    }

    #[tokio::test]
    async fn successful_daily_fetch_is_reused_but_failures_are_not_cached() {
        let cache = Mutex::new(QuantMarketCache::default());
        let key = DailyHistoryKey::new("000001", "cn", "2026-07-14", 21);
        let calls = Arc::new(AtomicUsize::new(0));

        let first_calls = Arc::clone(&calls);
        let first = get_or_fetch_daily(&cache, key.clone(), move || {
            first_calls.fetch_add(1, Ordering::SeqCst);
            async { Ok(vec![bar("2026-07-14", 10.0)]) }
        })
        .await
        .unwrap();
        let second_calls = Arc::clone(&calls);
        let second = get_or_fetch_daily(&cache, key, move || {
            second_calls.fetch_add(1, Ordering::SeqCst);
            async { Ok(vec![bar("2026-07-14", 11.0)]) }
        })
        .await
        .unwrap();

        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(first[0].close, second[0].close);

        let failed_key = DailyHistoryKey::new("000002", "cn", "2026-07-14", 21);
        let failed_calls = Arc::new(AtomicUsize::new(0));
        for _ in 0..2 {
            let failed_calls = Arc::clone(&failed_calls);
            assert!(get_or_fetch_daily(&cache, failed_key.clone(), move || {
                failed_calls.fetch_add(1, Ordering::SeqCst);
                async { Err("network failed".to_string()) }
            })
            .await
            .is_err());
        }
        assert_eq!(failed_calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn successful_intraday_fetch_reuses_bars_and_statistics() {
        let cache = Mutex::new(QuantMarketCache::default());
        let key = IntradayHistoryKey::new("000001", "cn", "2026-07-14", 22, 4, 4);
        let calls = Arc::new(AtomicUsize::new(0));

        let first_calls = Arc::clone(&calls);
        let first = get_or_fetch_intraday(
            &cache,
            key.clone(),
            move || {
                first_calls.fetch_add(1, Ordering::SeqCst);
                async { Ok(vec![bar("2026-07-13 09:30", 10.0)]) }
            },
            |_| IntradayTWindowStats {
                valid_days: 10,
                high_frequency_windows: vec!["09:30-09:45".to_string()],
                low_frequency_windows: vec![],
            },
        )
        .await
        .unwrap();
        let second_calls = Arc::clone(&calls);
        let second = get_or_fetch_intraday(
            &cache,
            key,
            move || {
                second_calls.fetch_add(1, Ordering::SeqCst);
                async { Ok(vec![bar("2026-07-13 09:30", 11.0)]) }
            },
            |_| IntradayTWindowStats::default(),
        )
        .await
        .unwrap();

        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(first.bars[0].close, second.bars[0].close);
        assert_eq!(first.stats, second.stats);
    }
}
