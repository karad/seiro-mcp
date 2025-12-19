---
layout: default
title: Runbook
lang: en
---

# Runbook: Start/Stop the MCP Server

## Purpose and scope

- Steps to start/stop `seiro-mcp` from Codex CLI / Inspector and exercise the visionOS tools (`validate_sandbox_policy` / `build_visionos_app` / `fetch_build_output`) within ~30 minutes.
- Target OS: macOS 15 / Linux 6.9+. visionOS builds require Xcode 16 + visionOS SDK.
- Assumes Rust 1.91.1, `cargo`, `bash`/`zsh`.

## Preparation

1. Dependency + build chain (fixed order)
   ```bash
   cargo run -p xtask -- preflight
   ```
   - Manual alternative:
     ```bash
     cargo fetch
     cargo check
     cargo test --all -- --nocapture
     cargo fmt -- --check
     cargo clippy -- -D warnings
     cargo build --release
     ```
2. Config: copy `config.example.toml` to `config.toml`, or point `MCP_CONFIG_PATH` to another path.
3. Token: set `MCP_SHARED_TOKEN` (16–128 chars) to match `[auth].token`, or pass via `--token`.
4. Refactor validation helpers (optional when working on Spec 008):
   ```bash
   cargo run -p xtask -- loc-guard
   cargo run -p xtask -- refactor-check-docs
   cargo run -p xtask -- api-baseline
   ```
   - `scripts/**` still exist as thin wrappers around `xtask`.

## Environment variables

### Required when launching from an MCP client

- `MCP_CONFIG_PATH` (required): absolute path to the TOML config file used by the server.
- `MCP_SHARED_TOKEN` (required): shared secret (16–128 chars) that must match `[auth].token` in config.

### Optional test/mocking helpers

These are intended for local development and tests; do not rely on them for production.

- `VISIONOS_TEST_TIME_SCALE`: time multiplier for mocked visionOS build steps (default: `1`).
- `VISIONOS_SANDBOX_PROBE`: sandbox probe backend (`env` for deterministic tests; otherwise uses OS commands).
- `VISIONOS_SANDBOX_SDKS`: comma-separated SDK list to simulate `xcodebuild -showsdks`.
- `VISIONOS_SANDBOX_DEVTOOLS`: simulate DevToolsSecurity status (`enabled`/`disabled`).
- `VISIONOS_SANDBOX_LICENSE`: simulate Xcode license status (`accepted`/`unlicensed`).
- `VISIONOS_SANDBOX_DISK_BYTES`: simulate available disk space in bytes.
- `VISIONOS_BUILD_ARTIFACT_DIR`: internal env set by the server when invoking `xcodebuild` (used by the mock script).

## How to launch

### Common options
- `--config` or `MCP_CONFIG_PATH`: **absolute path** to config. Default is `config.toml` in CWD.
- `--token` or `MCP_SHARED_TOKEN`: shared secret; CLI flag wins over env.
- `--transport {stdio|tcp}`: defaults to `stdio`; `tcp` listens on `server.host` / `server.port`.

### Inspector (stdio mode)
```bash
MCP_SHARED_TOKEN=<token> \
MCP_CONFIG_PATH=$PWD/config.toml \
npx @modelcontextprotocol/inspector target/release/seiro-mcp -- --transport=stdio
```
- Always launch via an MCP client to avoid `MCP_CLIENT_REQUIRED`.

### Inspector (tcp mode)
```bash
MCP_SHARED_TOKEN=<token> \
MCP_CONFIG_PATH=$PWD/config.toml \
npx @modelcontextprotocol/inspector mcp connect tcp://127.0.0.1:8787 -- \
  target/release/seiro-mcp --transport=tcp --config=$PWD/config.toml
```
- Startup fails on port conflicts (`EADDRINUSE`).

### Codex CLI example
```toml
[mcp_servers.operational]
command = "/Users/<user>/sources/repos/seiro-mcp/target/release/seiro-mcp"
args = ["--transport=stdio"]
env.MCP_CONFIG_PATH = "/Users/<user>/sources/repos/seiro-mcp/config.toml"
env.MCP_SHARED_TOKEN = "<token>"
working_directory = "/Users/<user>/sources/repos/seiro-mcp"
```
- For TCP, set `args = ["--transport=tcp"]` and align `server.host` / `server.port` in config.

## Stop flow and exit codes
- `Ctrl+C` (SIGINT) ends with exit code 0.
- Common failure exits:
  - 42: `AUTH_TOKEN_MISMATCH` (`[auth].token` mismatch)
  - 43: `MCP_TOKEN_REQUIRED` (missing token)
  - 44: `MCP_CLIENT_REQUIRED` (stdin/stdout is a TTY)
  - Missing config: `CONFIG_MISSING_FIELD` (exit 1), emitted as structured JSON on stderr.

## Troubleshooting

| Symptom / code | Resolution |
| --- | --- |
| `CONFIG_MISSING_FIELD` / `CONFIG_INVALID_FIELD` | Check required keys in `config.toml`; ensure `MCP_CONFIG_PATH` points to the intended file. |
| `AUTH_TOKEN_MISMATCH` (42) | Align `MCP_SHARED_TOKEN` or `--token` with `[auth].token`; spaces or short values fail. |
| `MCP_TOKEN_REQUIRED` (43) | Token missing. Provide a 16–128 char ASCII/UTF-8 value. |
| `MCP_CLIENT_REQUIRED` (44) | You ran `cargo run` directly. Launch via Inspector / Codex as a child process. |
| `artifact_expired` | Call `fetch_build_output` within TTL; raise `visionos.artifact_ttl_secs` if needed and document the retrieval flow. |
| TCP connect fail (`EADDRINUSE`) | Resolve port conflicts on `server.port` and retry. |

## Logs and telemetry
- All logs go to stderr. `RUST_LOG=rmcp_sample=info` (or higher) emits `RuntimeModeTelemetry` (transport, config_path, pending_jobs, etc.).
- Build job spans are under the `rmcp_sample::visionos` target. Enable JSON logs with `RUST_TRACING_FORMAT=json`.

## Manual verification
1. Run the build chain above (Clippy after TODO is resolved).
2. In Inspector stdio mode, confirm `mcp list` shows the visionOS tools.
3. Restart Codex CLI and confirm `mcp describe operational` shows all four tools.
4. In the visionOS mock flow, run `validate_sandbox_policy` → `build_visionos_app` → `fetch_build_output` (optionally set `MOCK_XCODEBUILD_BEHAVIOR`).
