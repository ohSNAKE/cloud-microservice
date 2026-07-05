-- 智能记账 AI 配置（示例模板，请复制为 setup-ai-config.local.sql 并填入真实 Key）
-- 数据库路径（macOS）：~/Library/Application Support/com.finance.assistant/finance.db
--
-- 执行方式：
--   sqlite3 "~/Library/Application Support/com.finance.assistant/finance.db" < scripts/setup-ai-config.local.sql

INSERT INTO settings (key, value) VALUES ('ai_enabled', 'true')
ON CONFLICT(key) DO UPDATE SET value = excluded.value;

INSERT INTO settings (key, value) VALUES ('ai_api_base', 'https://v2.pincc.ai/v1')
ON CONFLICT(key) DO UPDATE SET value = excluded.value;

INSERT INTO settings (key, value) VALUES ('ai_model', 'gpt-4o-mini')
ON CONFLICT(key) DO UPDATE SET value = excluded.value;

INSERT INTO settings (key, value) VALUES ('ai_api_key', 'YOUR_API_KEY_HERE')
ON CONFLICT(key) DO UPDATE SET value = excluded.value;
