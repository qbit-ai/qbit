# Tavily integration plan (search, extract, crawl, map)

This plan describes how to implement full Tavily API support in this repo, including **Search**, **Extract**, **Crawl**, and **Map**, and how to expose them as agent tools.

## Goals

- Implement first-class Tavily endpoints:
  - `POST https://api.tavily.com/search`
  - `POST https://api.tavily.com/extract`
  - `POST https://api.tavily.com/crawl`
  - `POST https://api.tavily.com/map`
- Ensure the agent can call these via tools (currently `web_search`, `web_search_answer`, `web_extract`).
- Include **all known request fields** from `tavily-notes.md` in our request structs / tool schemas.
- Keep API key handling centralized in `TavilyState`.

## Current state (what exists today)

- `backend/crates/qbit-web/src/tavily.rs`
  - `TavilyState::search(query, max_results)` uses `tavily::search()` with a limited subset of parameters.
  - `TavilyState::answer(query)` is a specialized search.
  - `TavilyState::extract(urls)` is **not** using Tavily `/extract`; it loops URLs and does a `site:` search with `include_raw_content=true`.
- `backend/crates/qbit-ai/src/tool_definitions.rs`
  - Tools: `web_search`, `web_search_answer`, `web_extract`.
- `backend/crates/qbit-ai/src/tool_executors.rs`
  - Executes those tools by calling `TavilyState` methods.

## Known request fields (from `tavily-notes.md`)

### Search (`POST /search`)

Known request JSON:

- `query: string`
- `search_depth: "basic" | "advanced"`
- `chunks_per_source: number`
- `max_results: number`
- `topic: "general" | ...` (string)
- `time_range: string | null`
- `start_date: string` (YYYY-MM-DD)
- `end_date: string` (YYYY-MM-DD)
- `include_answer: bool`
- `include_raw_content: bool`
- `include_images: bool`
- `include_image_descriptions: bool`
- `include_favicon: bool`
- `include_domains: string[]`
- `exclude_domains: string[]`
- `country: string | null`
- `auto_parameters: bool`
- `include_usage: bool`

### Extract (`POST /extract`)

Known request JSON:

- `urls: string | string[]` (notes show a string; API may accept array)
- `query: string`
- `chunks_per_source: number`
- `extract_depth: "basic" | "advanced"` (string)
- `include_images: bool`
- `include_favicon: bool`
- `format: "markdown" | "text" | ...` (string)
- `timeout: number | "None" | null` (notes show string "None")
- `include_usage: bool`

### Crawl (`POST /crawl`)

Known request JSON:

- `url: string`
- `instructions: string`
- `chunks_per_source: number`
- `max_depth: number`
- `max_breadth: number`
- `limit: number`
- `select_paths: string[] | null`
- `select_domains: string[] | null`
- `exclude_paths: string[] | null`
- `exclude_domains: string[] | null`
- `allow_external: bool`
- `include_images: bool`
- `extract_depth: "basic" | "advanced"`
- `format: "markdown" | ...`
- `include_favicon: bool`
- `timeout: number`
- `include_usage: bool`

### Map (`POST /map`)

Known request JSON:

- `url: string`
- `instructions: string`
- `max_depth: number`
- `max_breadth: number`
- `limit: number`
- `select_paths: string[] | null`
- `select_domains: string[] | null`
- `exclude_paths: string[] | null`
- `exclude_domains: string[] | null`
- `allow_external: bool`
- `timeout: number`
- `include_usage: bool`

## Step-by-step work plan

### 1) Decide client strategy (tavily crate vs direct HTTP)

1. Check whether the workspace `tavily` crate supports `/extract`, `/crawl`, `/map`.
2. If it does not (likely, given current comment in `tavily.rs`), implement these endpoints via `reqwest` directly inside `qbit-web`.
3. Prefer a single internal helper:
   - `async fn post_json<TReq: Serialize, TResp: DeserializeOwned>(endpoint: &str, req: &TReq) -> Result<TResp>`
   - Adds `Authorization: Bearer <token>` and `Content-Type: application/json`.

Deliverable: a clear decision and a consistent approach for all endpoints.

### 2) Add request/response types (serde) for all endpoints

Create Rust structs that include **all known fields** above.

- Location: `backend/crates/qbit-web/src/tavily.rs` (or split into `tavily/types.rs` if it grows).
- Use `#[derive(Serialize, Deserialize, Debug, Clone)]`.
- Use `Option<T>` for nullable fields.
- Use `#[serde(skip_serializing_if = "Option::is_none")]` for optional fields.
- For fields that can be either a string or array (`urls`), use an untagged enum:
  - `enum Urls { One(String), Many(Vec<String>) }`

Deliverable: `SearchRequest`, `ExtractRequest`, `CrawlRequest`, `MapRequest` plus response structs.

### 3) Implement TavilyState methods for each endpoint

In `backend/crates/qbit-web/src/tavily.rs`:

1. Keep `TavilyState` API key handling.
2. Implement:
   - `pub async fn search(&self, req: SearchRequest) -> Result<SearchResponse>`
   - `pub async fn extract(&self, req: ExtractRequest) -> Result<ExtractResponse>`
   - `pub async fn crawl(&self, req: CrawlRequest) -> Result<CrawlResponse>`
   - `pub async fn map(&self, req: MapRequest) -> Result<MapResponse>`
3. Decide whether to keep convenience wrappers:
   - `search(query, max_results)` can become a wrapper that builds `SearchRequest` with defaults.
   - `answer(query)` can become a wrapper that sets `include_answer=true` and `search_depth=advanced`.
4. Remove/replace the current “extract via site: search” workaround once `/extract` is implemented.

Deliverable: full endpoint coverage in `TavilyState`.

### 4) Update tool definitions to expose new capabilities

In `backend/crates/qbit-ai/src/tool_definitions.rs`:

1. Add tool definitions (names suggested):
   - `web_search` (already exists; expand schema to include all known request fields)
   - `web_extract` (expand schema to include all known request fields)
   - `web_crawl` (new)
   - `web_map` (new)
2. Ensure each tool’s JSON schema includes:
   - required vs optional fields
   - defaults (if the tool framework supports them) or document defaults in description
3. Keep gating: only register tools if `tavily_state.is_available()`.

Deliverable: tool schemas match the known request fields.

### 5) Update tool executors to call the new methods

In `backend/crates/qbit-ai/src/tool_executors.rs`:

1. Extend `execute_tavily_tool` match arms:
   - Parse tool args into the new request structs.
   - Call `tavily_state.search(req)` / `extract(req)` / `crawl(req)` / `map(req)`.
2. Standardize outputs:
   - Return raw Tavily response JSON (preferred) or map into existing `SearchResults`/etc.
   - Include `failed_urls` for extract if Tavily provides it; otherwise preserve current behavior.

Deliverable: tools execute end-to-end.

### 6) Ensure tool routing recognizes new tool names

In `backend/crates/qbit-ai/src/tool_execution.rs` (if this path is used):

1. Update `ToolCategory::from_tool_name` to include `web_crawl` and `web_map`.
2. Ensure the routed executor is not a placeholder (or ensure the app uses `tool_executors.rs`).

Deliverable: no dead-end routing for new tools.

### 7) Configuration: API key from settings (optional but recommended)

Currently `TavilyState::new()` only reads `TAVILY_API_KEY`.

1. Add a constructor like:
   - `pub fn from_api_key(api_key: Option<String>) -> Self`
   - or `pub fn from_settings(settings: &Settings) -> Self`
2. Update:
   - `backend/crates/qbit/src/state.rs`
   - `backend/crates/qbit/src/cli/bootstrap.rs`
   to prefer `settings.api_keys.tavily` and fall back to env.

Deliverable: consistent configuration behavior.

### 8) Tests

1. Unit tests (fast):
   - Serialize each request struct and snapshot/compare JSON keys.
   - Ensure optional fields are omitted when `None`.
   - Ensure `urls` untagged enum serializes correctly.
2. Integration tests (mocked HTTP):
   - Use `wiremock` or `httpmock` to simulate Tavily endpoints and verify:
     - correct URL
     - correct headers (Authorization)
     - correct request body
     - response parsing

Deliverable: confidence that requests match the known fields.

### 9) Verification

Run:

- `cargo fmt --all`
- `cargo clippy --all-targets --all-features -D warnings`
- `cargo test --all`

Deliverable: green build.

## Notes / open questions

- The notes show `timeout` as `"None"` for extract; we should confirm the real API accepts string values or use `Option<u64>` and omit when unset.
- Response shapes for `/extract`, `/crawl`, `/map` are not captured in `tavily-notes.md`; we will need to consult Tavily docs or inspect responses during implementation.
- Tool naming: current convention is `web_*` (not `tavily_*`). This plan follows existing naming.
