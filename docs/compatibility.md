---
layout: default
title: MCP client compatibility
lang: en
---

## MCP client compatibility

## Support scope

- Supported OS for building/testing the Rust workspace: macOS 15+ and Linux 6.9+.
- visionOS build tools (`validate_sandbox_policy`, `build_visionos_app`, `fetch_build_output`) require macOS + Xcode 16 with visionOS SDK installed.
- On non-macOS environments, the server can compile and tests can run (using mocks), but real visionOS builds are not supported.

## Compatibility policy

This repository treats the MCP surface as a contract:

- Tool names and input/output JSON shapes are stable.
- Error `code` values are stable and are considered part of the public API.
- Structured error metadata is stable:
  - `code`, `remediation`, `retryable`, `sandbox_state`
  - Additional fields may be added, but existing fields should not be removed or renamed.

### What counts as a breaking change

- Renaming/removing a tool or changing required request fields.
- Changing the meaning of an existing error `code`, or replacing a `code` with another one.
- Removing or renaming structured error fields (`code` / `retryable` / `sandbox_state` / `remediation`).

### Baselines and change detection

- Tool schemas live under `specs/**/contracts/*.json`.
- Contracts are guarded by integration tests (see `tests/integration/refactor_contracts.rs` and fixtures under `tests/fixtures/`).
- To capture a human-review baseline of contracts + CLI help output:
  ```bash
  cargo run -p xtask -- api-baseline specs/009-oss-release-prep/contracts/api-baseline.txt
  ```
  Review the diff and update docs/contracts when intentional.

### Supported libraries / protocol
- Uses `rmcp` crate v0.8.5; compatible with Model Context Protocol as of 2024Q2.
- Transports: `rmcp::transport::{stdio,tcp}`. Default is `stdio`; `--transport=tcp` listens on `server.host` / `server.port`.

### Verified clients
1. **Model Context Protocol Inspector CLI** (`npx @modelcontextprotocol/inspector`)
   - stdio:  
     ```bash
     MCP_SHARED_TOKEN=<token> MCP_CONFIG_PATH=$PWD/config.toml \
       npx @modelcontextprotocol/inspector target/release/seiro-mcp -- --transport=stdio
     ```
   - tcp:  
     ```bash
     MCP_SHARED_TOKEN=<token> MCP_CONFIG_PATH=$PWD/config.toml \
       npx @modelcontextprotocol/inspector mcp connect tcp://127.0.0.1:8787 -- \
         target/release/seiro-mcp --transport=tcp --config=$PWD/config.toml
     ```
   - Pass `MCP_CONFIG_PATH` via CLI `env` to run with any config file.
2. **Codex CLI**
   - Confirmed by adding to `~/.codex/config.toml` (use absolute paths; no env expansion):
     ```toml
     [mcp_servers.visionos]
     command = "/<this-repo-path>/target/release/seiro-mcp"
     args = ["--transport=stdio"]
     env.MCP_CONFIG_PATH = "/<this-repo-path>/config.toml"
     env.MCP_SHARED_TOKEN = "<token>"
     working_directory = "/<this-repo-path>"
     ```
   - After restarting Codex CLI, `mcp list` shows the visionOS tools. Build the release binary first.
   - For TCP, set `args = ["--transport=tcp"]` and align `server.host` / `server.port`.

### Compatibility checklist
- [ ] Use `target/release/seiro-mcp` from `cargo build --release`; visionOS tools respond via Inspector (stdio/tcp).
- [ ] Codex CLI `command` points to the binary (absolute path) and `env.MCP_CONFIG_PATH` / `env.MCP_SHARED_TOKEN` are set; tools register successfully.
- [ ] Missing token / invalid port, etc., are reproduced by unit tests in `src/server/config/mod.rs`, blocking client startup.

### Known limitations
- Running `cargo run` directly exits immediately because no MCP client is attached (`MCP_CLIENT_REQUIRED` exit 44). Always launch via an MCP client.
- No support for transports other than stdio/tcp (e.g., WebSocket).
- Without visionOS SDK, `validate_sandbox_policy` returns `sdk_missing`; visionOS tools are unavailable on Linux.
