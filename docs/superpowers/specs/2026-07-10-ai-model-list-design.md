# AI Model List Design

## Goal

Add model list retrieval to the existing AI settings card. Users should be able to fetch available model IDs from the configured OpenAI-compatible provider and select one without losing the ability to type a custom model name.

## Current Context

- `src/pages/Settings.tsx` stores `ai_api_key`, `ai_api_base`, and `ai_model` through the existing settings form.
- `src-tauri/src/services/ai_parser.rs` already calls `{ai_api_base}/chat/completions` with `Authorization: Bearer <key>`.
- The current model field is a plain text input.

## Approach

Use a Tauri backend command to call the provider's OpenAI-compatible model endpoint: `GET {api_base}/models`.

The frontend will pass the current, unsaved form values for API base and key to the command. The command returns a sorted list of model IDs parsed from `data[].id`.

## UI Behavior

- Replace the model text input with an Ant Design `AutoComplete` backed by fetched model options. This keeps `ai_model` as a single `string` while still allowing arbitrary manual input.
- Add a `获取模型` button next to the model field.
- Disable model fetch when AI settings are disabled.
- Show loading while fetching.
- On success, populate the dropdown and keep the current selected value.
- If the provider returns no models, show a warning and keep manual entry available.
- On failure, show the backend error message and keep manual entry available.

## Backend Behavior

- Add `list_ai_models_command(api_base, api_key) -> Result<Vec<String>, String>`.
- Trim trailing slashes from `api_base`, call `{base}/models`, and send the bearer token.
- Parse OpenAI-compatible responses shaped like `{ "data": [{ "id": "..." }] }`.
- Validate missing base/key and return user-facing Chinese errors.
- Do not persist fetched models. Only `ai_model` remains persisted through existing settings.
- Register the command in `src-tauri/src/lib.rs` and re-export it through the existing `commands` module if needed.
- Add `api.listAiModels(apiBase, apiKey)` in `src/api/index.ts`; invoke arguments should use Tauri's camelCase frontend shape: `{ apiBase, apiKey }`.

## Testing

- TypeScript typecheck should pass.
- Rust tests/build checks should pass.
- Add focused Rust helper tests for model response parsing, empty model lists, missing base/key validation, and malformed provider responses where practical without network access.
- Manual behavior can be verified from settings by entering a provider base/key, clicking `获取模型`, selecting a returned model, and saving settings.
