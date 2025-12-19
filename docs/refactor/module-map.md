# Refactor Module Map (lib → tools → server)

This page is the fast navigation entry for the refactored Rust module layout.

## Top-level structure

- `src/lib/`: shared helpers with single responsibility (no dependencies on `tools/` or `server/`).
- `src/tools/`: tool implementations and feature modules (depends on `src/lib/`).
- `src/server/`: MCP server runtime wiring (depends on `src/tools/` and `src/lib/`).
- `src/cli/`: CLI args and launch profile parsing (used by `src/main.rs`).

## 2-step navigation (find the entry in <=2 hops)

### MCP server start

1. `src/main.rs` → `seiro_mcp::server::runtime::run_server`
2. `src/server/runtime/startup.rs` → creates `VisionOsServer` and serves stdio/TCP

### Tool registry

1. `src/server/runtime/tool_registry.rs` → `VisionOsServer` (`tool_router` + tool handlers)
2. `src/tools/mod.rs` / `src/tools/visionos/registry.rs` → per-tool registration

### visionOS build tool

1. `src/tools/visionos/build/mod.rs` → public API (`run_build`, request/response types)
2. `src/tools/visionos/build/executor.rs` → executes `xcodebuild` and zips artifacts

### Sandbox validation tool

1. `src/tools/visionos/sandbox/mod.rs` → `validate_sandbox_policy`
2. `src/tools/visionos/sandbox/probe.rs` → probe abstraction (system vs env)

### Artifact store

1. `src/tools/visionos/artifacts/mod.rs` → `fetch_build_output` entry point
2. `src/tools/visionos/artifacts/store.rs` → in-memory store + TTL behavior

## Notes

- Keep `src/lib/` modules generic and reusable; avoid importing `tools`/`server` types in `lib`.
- Use `scripts/refactor/loc_guard.sh` to keep LOC targets honest.
