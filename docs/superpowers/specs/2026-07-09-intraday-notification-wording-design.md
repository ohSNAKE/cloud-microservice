# Intraday Notification Wording Design

## Goal

Make intraday T desktop notifications describe the trigger direction clearly.

## Approved Copy

- Sell T: `历史高点高发时段 {window}，当前日内位置 {percent}%，可关注卖T。仅供参考。`
- Buyback: `历史低点高发时段 {window}，当前日内位置 {percent}%，可关注买回。仅供参考。`

## Scope

- Update backend-generated `notification_body` for persisted/generated intraday T signals.
- Update frontend fallback notification body for intraday T signals when backend body is absent.
- Do not change signal rules, thresholds, cooldown, or persistence.

## Verification

- Backend unit tests assert direction-specific notification wording.
- Frontend typecheck and production build pass.
