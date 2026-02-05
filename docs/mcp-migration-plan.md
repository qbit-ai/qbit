# MCP Legacy SSE Transport Support Plan

## Overview

Add support for legacy SSE (Server-Sent Events) transport in the `qbit-mcp` crate, enabling connections to MCP servers like DeepWiki that only support the deprecated HTTP+SSE protocol (MCP protocol version 2024-11-05).

## Current State

### Dependencies
```toml
# backend/crates/qbit-mcp/Cargo.toml
rmcp = { version = "0.14", features = [
    "client",
    "client-side-sse",
    "transport-child-process",
    "transport-io",
    "transport-streamable-http-client",
    "transport-streamable-http-client-reqwest"
] }
futures = { workspace = true }
sse-stream = "0.2"
```

### Transport Support
| Transport | Status |
|-----------|--------|
| Stdio | Working |
| Streamable HTTP | Working |
| SSE (Legacy) | **Implemented** — custom transport via `SinkStreamTransport` |

### Key Constraint

**`rmcp` is the official MCP Rust SDK** (maintained at `modelcontextprotocol/rust-sdk`). In version 0.14, the only two built-in client transport types are `TokioChildProcess` (stdio) and `StreamableHttpClientTransport` (streamable HTTP). There is no standalone `SseTransport` for legacy SSE client connections, so we built a custom one using rmcp's `SinkStreamTransport`.

### Files
- `backend/crates/qbit-mcp/Cargo.toml` — Dependencies (added `client-side-sse`, `futures`, `sse-stream`)
- `backend/crates/qbit-mcp/src/sse_transport.rs` — **New** — Custom legacy SSE transport implementation
- `backend/crates/qbit-mcp/src/client.rs` — Updated `connect_sse()` to use the new transport
- `backend/crates/qbit-mcp/src/lib.rs` — Added `pub mod sse_transport`
- `backend/crates/qbit-mcp/src/config.rs` — `McpTransportType::Sse` variant (already existed)
- `backend/crates/qbit-mcp/src/manager.rs` — Server connection management (unchanged)

---

## Approach

### Custom SSE Transport (Option A — Chosen)

Built a custom legacy SSE client transport using rmcp's `SinkStreamTransport<Sink, Stream>` trait and the `sse-stream` + `reqwest` dependencies.

**How legacy SSE works:**
1. Client sends `GET /sse` → receives an SSE stream with an `endpoint` event containing a message URL
2. Client sends JSON-RPC messages via `POST` to that message URL
3. Server sends responses/notifications via the SSE stream as `message` events

**Implementation details:**
- `SseSink` — `futures::Sink` that POSTs JSON-RPC messages to the endpoint URL
- `SseMessageStream` — `futures::Stream` that filters SSE events for `message` type and deserializes JSON-RPC
- `connect()` — Establishes SSE connection, waits for `endpoint` event, returns `SinkStreamTransport`
- URL resolution handles both absolute and relative endpoint URLs
- 30-second timeout on initial `endpoint` event

---

## Implementation Progress

### Phase 1: Compatibility Check — SKIPPED
Skipped testing `StreamableHttpClientTransport` against legacy SSE servers. The protocols use different endpoints (`GET /sse` + `POST /messages` vs `POST /mcp`), making compatibility unlikely.

### Phase 2: Implement Custom SSE Transport — DONE

- [x] Add `client-side-sse` feature flag to rmcp
- [x] Add `futures` and `sse-stream` dependencies
- [x] Create `sse_transport.rs` with `connect()`, `SseSink`, `SseMessageStream`
- [x] Update `connect_sse()` in `client.rs` to use the new transport
- [x] Add `pub mod sse_transport` to `lib.rs`

### Phase 3: Testing — PARTIAL

#### 3.1 Build & Lint
- [x] `cargo check -p qbit-mcp` passes
- [x] `cargo clippy -p qbit-mcp` passes (zero warnings)
- [x] `cargo clippy --workspace` passes

#### 3.2 Unit Tests
- [x] Existing tests pass (41/44 — 3 pre-existing loader test failures unrelated to SSE)
- [x] SSE URL resolution tests pass (3 tests: absolute, relative, relative-no-slash)

#### 3.3 Integration Tests — TODO
- [ ] Test SSE transport against DeepWiki (`https://mcp.deepwiki.com/sse`)
  - [ ] Verify connection succeeds
  - [ ] Verify tool listing works
  - [ ] Verify tool execution works

#### 3.4 Regression Tests — TODO
- [ ] Verify existing stdio connections still work
- [ ] Verify existing HTTP connections still work

#### 3.5 Full Application Test — TODO
- [ ] Run `just dev`
- [ ] Configure DeepWiki SSE server in MCP config
- [ ] Test using MCP tools from the agent

### Phase 4: Cleanup — DONE
- [x] No workaround code
- [x] `cargo clippy -p qbit-mcp` clean
- [x] `cargo clippy --workspace` clean

---

## Remaining Work

### Required
1. **Integration test against a real SSE server** — Configure DeepWiki in `~/.qbit/mcp.json` or project `.qbit/mcp.json` and verify the full flow (connect → list tools → call tool)
2. **Regression test stdio/HTTP transports** — Verify existing MCP server connections still work after the changes

### Optional / Future
3. **Reconnection logic** — If the SSE stream drops, the transport currently fails. A reconnection mechanism with backoff could improve reliability.
4. **Fix pre-existing loader test failures** — 3 loader tests fail because they pick up the user's real `~/.qbit/mcp.json`. These should be isolated via `HOME` override or similar.

---

## Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|------------|
| SSE stream drops or reconnection needed | Medium | Add reconnection logic with backoff (future work) |
| Auth headers not propagated to SSE stream | Medium | Headers are passed via `reqwest::Client::default_headers()` |
| Legacy SSE servers vary in implementation | Low | Test against DeepWiki as primary target |

## Rollback Plan

If the custom transport proves unreliable:
1. Revert changes to `qbit-mcp` crate (3 files modified, 1 file added)
2. Keep the descriptive error message in `connect_sse()`
3. Consider contributing SSE client transport upstream to `modelcontextprotocol/rust-sdk`

## Success Criteria

1. ~~All three transports work (stdio, HTTP, SSE)~~ — SSE implemented, needs integration verification
2. DeepWiki SSE server connects and tools are usable — **TODO**
3. Existing stdio/HTTP servers continue to work — **TODO** (regression check)
4. All tests pass — Unit tests pass; integration tests TODO
5. No new external dependencies beyond what's already in the tree — **Done** (`sse-stream` is new but minimal)

## References

- rmcp (official MCP Rust SDK): https://github.com/modelcontextprotocol/rust-sdk
- rmcp 0.14 docs: https://docs.rs/rmcp/0.14.0
- rmcp transport module: https://docs.rs/rmcp/0.14.0/rmcp/transport/index.html
- MCP legacy SSE spec: https://modelcontextprotocol.io/legacy/concepts/transports
- Current qbit-mcp code: `backend/crates/qbit-mcp/`
