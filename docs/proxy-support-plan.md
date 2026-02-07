# HTTP/HTTPS Proxy Support — Implementation Plan

## Overview

Add HTTP/HTTPS proxy client support for all LLM providers and outbound HTTP requests in Qbit,
with a UI settings panel for configuration.

## Architecture

Three tiers of HTTP client control exist:

| Tier | Providers | Proxy Strategy |
|------|-----------|----------------|
| rig-core built-in | OpenAI, Anthropic, OpenRouter, Gemini, Groq, xAI, Ollama | Switch from `Client::new()` to `Client::builder().http_client(client).build()` |
| Custom rig-* crates | rig-anthropic-vertex, rig-gemini-vertex, rig-zai-sdk | Accept optional pre-configured `reqwest::Client` in constructors |
| async-openai wrapper | rig-openai-responses | Use `OpenAIClient::with_config().with_http_client()` |

rig-core 0.29 exposes `.http_client()` on its `ClientBuilder` (confirmed in source at
`~/.cargo/registry/.../rig-core-0.29.0/src/client/mod.rs:530`).

## Implementation Steps

### Phase 1: Settings Schema (Backend + Frontend types)

1. Add `ProxySettings` struct to `qbit-settings/src/schema.rs`
2. Add `proxy: ProxySettings` field to `QbitSettings`
3. Add `[proxy]` section to `qbit-settings/src/template.toml`
4. Add `ProxySettings` interface to `frontend/lib/settings.ts`
5. Add `proxy` to `QbitSettings` interface and `DEFAULT_SETTINGS`

### Phase 2: HTTP Client Factory (Backend)

6. Create `qbit-llm-providers/src/http_client.rs` — shared `build_http_client(proxy) -> Result<reqwest::Client>`
7. Enable `socks` feature on reqwest in `backend/Cargo.toml` for SOCKS5 proxy support
8. Re-export from `qbit-llm-providers/src/lib.rs`

### Phase 3: Provider Integration (Backend)

9. Update `qbit-llm-providers/src/provider_trait.rs`:
   - Add `reqwest::Client` param to `LlmProvider::create_client()`
   - Add `reqwest::Client` param to `create_provider()` and `create_client_for_model()`
   - Switch all rig-core providers from `Client::new()` to `Client::builder().http_client().build()`

10. Update custom rig-* crate constructors:
    - `rig-anthropic-vertex/src/client.rs` — accept `Option<reqwest::Client>`
    - `rig-gemini-vertex/src/client.rs` — accept `Option<reqwest::Client>`
    - `rig-zai-sdk/src/client.rs` — accept `Option<reqwest::Client>` (crate is `rig-zai-sdk` on disk)

11. Update `rig-openai-responses/src/completion.rs`:
    - Add `Client::with_http_client()` constructor

12. Update `qbit-ai/src/llm_client.rs`:
    - Build shared `reqwest::Client` from `ProxySettings` via factory
    - Pass to all `create_*_components()` functions
    - Pass through `LlmClientFactory`

13. Update `qbit-ai/src/agent_bridge.rs`:
    - Thread proxy-configured client through agent bridge initialization

### Phase 4: Non-LLM HTTP Clients (Backend, optional but recommended)

14. Update `qbit-web/src/tavily.rs` — accept shared client
15. Update `qbit-web/src/web_fetch.rs` — accept shared client
16. Update `qbit-mcp/src/client.rs` — accept shared client

### Phase 5: UI Settings

17. Update `frontend/components/Settings/AdvancedSettings.tsx`:
    - Add `proxy` prop and `onProxyChange` callback
    - Render Proxy card with URL, username, password, no_proxy fields

18. Update `frontend/components/Settings/index.tsx`:
    - Pass proxy settings to AdvancedSettings

19. Update `frontend/components/Settings/SettingsTabContent.tsx`:
    - Mirror the same wiring

## File Change Summary

| File | Operation |
|------|-----------|
| `backend/crates/qbit-settings/src/schema.rs` | Modify — add `ProxySettings` struct |
| `backend/crates/qbit-settings/src/template.toml` | Modify — add `[proxy]` section |
| `backend/crates/qbit-llm-providers/src/http_client.rs` | Create — shared client factory |
| `backend/crates/qbit-llm-providers/src/lib.rs` | Modify — re-export http_client |
| `backend/crates/qbit-llm-providers/src/provider_trait.rs` | Modify — add client param everywhere |
| `backend/crates/rig-anthropic-vertex/src/client.rs` | Modify — accept optional client |
| `backend/crates/rig-gemini-vertex/src/client.rs` | Modify — accept optional client |
| `backend/crates/rig-zai-sdk/src/client.rs` | Modify — accept optional client |
| `backend/crates/rig-openai-responses/src/completion.rs` | Modify — add with_http_client constructor |
| `backend/crates/qbit-ai/src/llm_client.rs` | Modify — build & thread shared client |
| `backend/crates/qbit-ai/src/agent_bridge.rs` | Modify — thread client |
| `backend/Cargo.toml` | Modify — enable reqwest socks feature |
| `frontend/lib/settings.ts` | Modify — add ProxySettings type |
| `frontend/components/Settings/AdvancedSettings.tsx` | Modify — add proxy UI section |
| `frontend/components/Settings/index.tsx` | Modify — wire proxy props |
| `frontend/components/Settings/SettingsTabContent.tsx` | Modify — wire proxy props |

## Testing Strategy (TDD)

- **Unit tests for `build_http_client()`** — verify proxy, auth, no_proxy, and default behavior
- **Unit tests for `ProxySettings` serde** — verify TOML round-trip, defaults, skip_serializing_if
- **Frontend tests for `AdvancedSettings`** — verify proxy fields render and onChange fires
- **Frontend tests for settings types** — verify DEFAULT_SETTINGS includes proxy
