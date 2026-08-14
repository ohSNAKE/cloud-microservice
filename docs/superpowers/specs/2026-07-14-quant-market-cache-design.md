# Quant Market Cache Design

## Goal

Reduce repeated market-data requests and repeated intraday-T window analysis during quant refreshes without changing realtime signal validation, notification behavior, or the frontend's current refresh cadence.

## Scope

- Cache daily K-lines per stock, market, and China trading date.
- Cache intraday-T historical 5-minute K-lines and their derived high/low frequency-window statistics per stock, market, trading date, and relevant intraday settings.
- Keep realtime quotes uncached and fetched every refresh.
- Limit concurrent snapshot fetches across targets.

## Architecture

Add an in-memory quant market cache owned by the snapshot-loading path. It is keyed by code, market, period, and China trading date. Intraday-T statistic entries additionally include lookback days and high/low minimum-count settings.

On a cache miss, the snapshot loader fetches the historical K-line data and stores only a successful result. On a hit, it reuses the historical bars and, for intraday-T targets, the precomputed window statistics. Realtime quote fetching and strict quote validation remain outside the cache.

The cache exists only for the running application process. No SQLite tables, migrations, or persistent cache invalidation are introduced.

## Invalidation

- A new China trading date produces different cache keys and naturally bypasses prior-day entries.
- Changing strategy mode, intraday lookback days, or high/low time-window thresholds changes the intraday cache key.
- Failed network requests are never cached.
- Manual and scheduled refreshes use the same cache. Manual refresh does not bypass valid same-day historical cache entries.

## Concurrency And Errors

- Snapshot loading uses a fixed concurrency limit across targets to avoid bursting the quote provider.
- The daily and 5-minute historical fetches may be served from cache; the realtime quote still executes for every target refresh.
- A cached historical result may continue to populate display windows when a current realtime quote is stale or unavailable, but strict quote validation continues to prevent new signals.
- Current-day 5-minute bars remain used only for intraday position and confirmation, while historical window statistics continue to exclude the current date.

## Verification

- Unit-test daily and 5-minute cache hits, date/settings invalidation, and failed-request non-caching.
- Verify cached intraday-T statistics equal the existing uncached calculation.
- Verify realtime quotes are never cached.
- Verify the target concurrency limit.
- Run Rust library tests, frontend tests, type checking, and production build.
