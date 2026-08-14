# AI Model List Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add provider model-list fetching to AI settings so users can retrieve and select OpenAI-compatible model IDs.

**Architecture:** Add a small backend command that calls `GET {api_base}/models`, parses `data[].id`, and returns `Vec<String>`. Add a frontend API wrapper and replace the model text input with an editable Ant Design `AutoComplete` plus a fetch button.

**Tech Stack:** Tauri 2, Rust, `ureq`, React 19, TypeScript, Ant Design.

---

## File Structure

- Modify `src-tauri/src/commands/ai.rs`: add model response DTOs, parsing/validation helpers, tests, and Tauri command.
- Modify `src-tauri/src/commands/mod.rs`: ensure the new command is exported if the module uses explicit re-exports.
- Modify `src-tauri/src/lib.rs`: register the new command in `generate_handler!`.
- Modify `src/api/index.ts`: add `api.listAiModels(apiBase, apiKey)` wrapper using camelCase invoke args.
- Modify `src/pages/Settings.tsx`: add model options/loading state, fetch handler, and `AutoComplete` UI.

## Chunk 1: Backend Model List Command

### Task 1: Parse and Validate Provider Models

**Files:**
- Modify: `src-tauri/src/commands/ai.rs`

- [ ] **Step 1: Add failing helper tests**

Add tests near the end of `src-tauri/src/commands/ai.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ai_models_keeps_non_empty_ids_sorted() {
        let json = r#"{"data":[{"id":"gpt-4o"},{"id":""},{"id":"gpt-4o-mini"},{"id":"gpt-4o"}]}"#;
        assert_eq!(parse_ai_model_ids(json).unwrap(), vec!["gpt-4o", "gpt-4o-mini"]);
    }

    #[test]
    fn parse_ai_models_allows_empty_data() {
        assert_eq!(parse_ai_model_ids(r#"{"data":[]}"#).unwrap(), Vec::<String>::new());
    }

    #[test]
    fn parse_ai_models_rejects_malformed_json() {
        assert!(parse_ai_model_ids("not json").is_err());
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml parse_ai_models`

Run: `cargo test --manifest-path src-tauri/Cargo.toml validate_ai_model_request`

Expected: FAIL because helper functions do not exist.

- [ ] **Step 3: Add minimal parser and validator helpers**

In `src-tauri/src/commands/ai.rs`, add DTOs and helpers:

```rust
#[derive(Debug, serde::Deserialize)]
struct AiModelsResponse {
    data: Vec<AiModelItem>,
}

#[derive(Debug, serde::Deserialize)]
struct AiModelItem {
    id: String,
}

fn parse_ai_model_ids(body: &str) -> Result<Vec<String>, String> {
    let parsed: AiModelsResponse = serde_json::from_str(body)
        .map_err(|e| format!("模型列表响应解析失败: {e}"))?;
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
```

- [ ] **Step 4: Run helper tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml parse_ai_models`

Run: `cargo test --manifest-path src-tauri/Cargo.toml validate_ai_model_request`

Expected: PASS.

### Task 2: Add Tauri Command

**Files:**
- Modify: `src-tauri/src/commands/ai.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add command implementation**

Add to `src-tauri/src/commands/ai.rs`:

```rust
#[tauri::command]
pub async fn list_ai_models_command(api_base: String, api_key: String) -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let (base, key) = validate_ai_model_request(&api_base, &api_key)?;
        let url = format!("{base}/models");
        let response = ureq::AgentBuilder::new()
            .timeout_connect(std::time::Duration::from_secs(15))
            .timeout_read(std::time::Duration::from_secs(45))
            .build()
            .get(&url)
            .set("Authorization", &format!("Bearer {key}"))
            .call()
            .map_err(|e| format!("模型列表请求失败: {e}"))?;

        if !(200..300).contains(&response.status()) {
            let status = response.status();
            let err_body = response.into_string().unwrap_or_default();
            return Err(format!("模型列表接口错误 ({status}): {err_body}"));
        }

        let body = response
            .into_string()
            .map_err(|e| format!("模型列表响应读取失败: {e}"))?;
        parse_ai_model_ids(&body)
    })
    .await
    .map_err(|e| e.to_string())?
}
```

- [ ] **Step 2: Register command**

Add `commands::list_ai_models_command` to `src-tauri/src/lib.rs` `generate_handler!` near `parse_transaction_nl_command`.

If `src-tauri/src/commands/mod.rs` explicitly exports commands, export `ai::list_ai_models_command` there.

- [ ] **Step 3: Run backend checks**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: PASS unless the repository has a pre-existing unrelated Rust compile blocker. If compilation fails in unrelated quant code, record the blocker and verify the new helpers with the focused commands above after confirming the failure is unrelated to the AI model-list changes.

## Chunk 2: Frontend Settings Integration

### Task 3: Add Frontend API Wrapper

**Files:**
- Modify: `src/api/index.ts`

- [ ] **Step 1: Add wrapper**

Add near existing AI parse/settings calls:

```ts
listAiModels: (apiBase: string, apiKey: string) =>
  invoke<string[]>("list_ai_models_command", { apiBase, apiKey }),
```

- [ ] **Step 2: Run typecheck**

Run: `npm run typecheck`

Expected: PASS or no new errors from this change.

### Task 4: Add Editable Model List UI

**Files:**
- Modify: `src/pages/Settings.tsx`

- [ ] **Step 1: Update imports and state**

Import `AutoComplete` and add state:

```ts
const [aiModelOptions, setAiModelOptions] = useState<{ value: string }[]>([]);
const [aiModelsLoading, setAiModelsLoading] = useState(false);
```

- [ ] **Step 2: Add fetch handler**

```ts
const handleFetchAiModels = async () => {
  const apiBase = form.getFieldValue("ai_api_base")?.trim();
  const apiKey = form.getFieldValue("ai_api_key")?.trim();
  const currentModel = form.getFieldValue("ai_model")?.trim();

  setAiModelsLoading(true);
  try {
    const models = await api.listAiModels(apiBase, apiKey);
    const values = currentModel && !models.includes(currentModel) ? [currentModel, ...models] : models;
    setAiModelOptions(values.map((value) => ({ value })));
    if (models.length === 0) {
      message.warning("供应商未返回可用模型，可继续手动填写");
    } else {
      message.success(`已获取 ${models.length} 个模型`);
    }
  } catch (e) {
    message.error(`获取模型失败：${String(e)}`);
  } finally {
    setAiModelsLoading(false);
  }
};
```

- [ ] **Step 3: Replace model input**

Use an outer labeled `Form.Item` and bind the `AutoComplete` through an inner `noStyle` item so AntD Form passes `value` and `onChange` to the real control:

```tsx
<Form.Item label="模型">
  <Input.Group compact>
    <Form.Item name="ai_model" noStyle>
      <AutoComplete
        options={aiModelOptions}
        placeholder="gpt-4o-mini"
        disabled={!aiEnabled}
        style={{ width: "calc(100% - 96px)" }}
        filterOption={(inputValue, option) =>
          String(option?.value ?? "").toLowerCase().includes(inputValue.toLowerCase())
        }
      />
    </Form.Item>
    <Button loading={aiModelsLoading} disabled={!aiEnabled} onClick={handleFetchAiModels}>
      获取模型
    </Button>
  </Input.Group>
</Form.Item>
```

- [ ] **Step 4: Run typecheck**

Run: `npm run typecheck`

Expected: PASS.

## Final Verification

- [ ] Run `npm run typecheck`.
- [ ] Run `cargo test --manifest-path src-tauri/Cargo.toml`; if it fails in pre-existing unrelated quant code, record that blocker and include the focused AI helper test results instead.
- [ ] Run `npm run build` if time allows to verify production frontend build.
- [ ] Manually inspect settings page behavior in dev if a provider key is available.

## Commit Guidance

Do not create a git commit unless the user explicitly requests one.
