---
layout: default
title: Quickstart
lang: en
---

# Quickstart

This repository ships the visionOS build MCP server. Follow these steps to finish `cargo check` → `cargo test --all` → `cargo fmt -- --check` → `cargo build --release` within ~30 minutes on a fresh machine and call the three tools (`validate_sandbox_policy` / `build_visionos_app` / `fetch_build_output`) from an MCP client.

## Prerequisites

- macOS 15 Sequoia or later
- Xcode 16+ with visionOS / visionOS Simulator SDK
- Rust 1.91.1 (`rustup override set 1.91.1`)
- `cargo`, `git`, `bash`/`zsh`
- An MCP client (Codex CLI or official Inspector)


## Installation

### 1. Clone this repository

```bash
$ git clone git@github.com:karad/seiro-mcp.git
```

### 2. Fetch dependencies

```bash
cargo fetch
```

### 3. Prepare `config.toml`

(see [`docs/config.md`](docs/config.md) for details)

- Copy from `config.example.toml` as a starting point.
  ```toml
  [server]
  host = "127.0.0.1"
  port = 8787
  
  [auth]
  token = "change-me-please"
  
  [visionos]
  allowed_paths = []
  allowed_schemes = []
  default_destination = "platform=visionOS Simulator,name=Apple Vision Pro"
  required_sdks = ["visionOS", "visionOS Simulator"]
  xcode_path = "/Applications/Xcode.app/Contents/Developer"
  xcodebuild_path = "/usr/bin/xcodebuild"
  max_build_minutes = 20
  artifact_ttl_secs = 600
  cleanup_schedule_secs = 60
  ```
- Update `token` to 16+ characters.
- To use another path, set `MCP_CONFIG_PATH=/path/to/config.toml`.
- In `[visionos]`, list at least one absolute path in `allowed_paths` and control build timeout / artifact TTL.
- `allowed_schemes` must list Xcode schemes allowed to build; anything else returns `scheme_not_allowed`.

#### Switching configs with `MCP_CONFIG_PATH`

- When separating dev/prod configs, add `MCP_CONFIG_PATH` to the launch environment:
  ```bash
  MCP_CONFIG_PATH=$PWD/config.toml cargo run --quiet
  ```
- MCP clients (e.g., Codex CLI) can pass `env.MCP_CONFIG_PATH` as well.
- Behavior is covered in `src/server/config/mod.rs::tests::load_config_from_env_override`.

##### Run the build and checks

Recommended (runs the minimum local quality gate):
```bash
cargo run -p xtask -- preflight
```

Manual alternative:
```bash
cargo fetch
cargo check
cargo test --all -- --nocapture
cargo fmt -- --check
cargo clippy -- -D warnings
cargo build --release
```

Additional repository checks:
```bash
cargo run -p xtask -- langscan
cargo run -p xtask -- docs-langscan
cargo run -p xtask -- check-docs-links
cargo run -p xtask -- loc-baseline
cargo run -p xtask -- loc-guard
cargo run -p xtask -- api-baseline
```


If any step fails, fix and rerun.
- On success, `target/release/seiro-mcp` is produced and can be referenced by MCP clients.


## Using from Codex CLI

Add an entry like the following to Codex CLI config (`~/.codex/config.toml`) to call the visionOS tools:

```toml
[mcp_servers.visionos_build]
command = "/<this-repo-path>/target/release/seiro-mcp"
args = ["--transport=stdio"]
env.MCP_CONFIG_PATH = "/<this-repo-path>/config.toml"
env.MCP_SHARED_TOKEN = "change-me-please"
working_directory = "/<this-repo-path>"
```

- Codex CLI does not expand `${HOME}`, so use absolute paths and replace `<your-username>`.
- Run `cargo build --release` beforehand so `target/release/seiro-mcp` exists.
- Switch server configs via `env.MCP_CONFIG_PATH`; ensure `env.MCP_SHARED_TOKEN` matches `[auth].token`.
- Restart Codex CLI and confirm `mcp list` shows the visionOS tools.


## How It Works

### 1. Launch the server via an MCP client

 - The MCP client must spawn the server as a child process and perform the RMCP handshake over stdio. Running `cargo run` directly without a client will fail immediately.
 - Example with Inspector:
   ```bash
   MCP_SHARED_TOKEN=<shared-token> MCP_CONFIG_PATH=$PWD/config.toml \
     npx @modelcontextprotocol/inspector cargo run --quiet -- --transport=stdio
   ```
 - When registering with Codex CLI or other editor extensions, also run `cargo run --quiet` as the subprocess.

### 2. Validate sandbox policy before building

```bash
mcp call validate_sandbox_policy '{
    "project_path": "/Users/<user>/codex/workspaces/vision-app",
    "required_sdks": ["visionOS", "visionOS Simulator"],
    "xcode_path": "/Applications/Xcode.app/Contents/Developer"
}'
```
- If `status: "ok"`, proceed to `build_visionos_app`.
- If `status: "error"` or an MCP error, fix based on the code:
    - `path_not_allowed`: add the project parent directory to `visionos.allowed_paths`.
    - `sdk_missing`: install visionOS SDK from Xcode > Settings > Platforms.
    - `devtools_security_disabled`: run `DevToolsSecurity -enable`.
    - `xcode_unlicensed`: run `sudo xcodebuild -license`.
    - `disk_insufficient`: ensure 20GB+ free space for the build.

### 3. Start a build with `build_visionos_app`

```bash
mcp call build_visionos_app '{
    "project_path": "/Users/<user>/codex/workspaces/VisionApp/VisionApp.xcodeproj",
    "scheme": "VisionApp",
    "destination": "platform=visionOS Simulator,name=Apple Vision Pro",
    "configuration": "Debug",
    "extra_args": ["-quiet"],
    "env_overrides": {"MOCK_XCODEBUILD_BEHAVIOR": "success"}
}'
```
- `project_path` / `workspace` must be absolute paths within `visionos.allowed_paths`.
- `scheme` must be listed in `visionos.allowed_schemes`.
- Allowed `extra_args`: `-quiet`, `-UseModernBuildSystem=YES`, `-skipPackagePluginValidation`, `-allowProvisioningUpdates`.
- `MOCK_XCODEBUILD_BEHAVIOR` switches the test fixture (`tests/fixtures/visionos/mock-xcodebuild.sh`) among `success` / `fail` / `timeout`.
- On success, returns `job_id`, `artifact_path`, `artifact_sha256`, `log_excerpt`, `duration_ms`; on failure, returns errors such as `build_failed` or `timeout`.

### 4. Download artifacts with `fetch_build_output`

```bash
mcp call fetch_build_output '{
    "job_id": "<UUID returned by build_visionos_app>",
    "include_logs": true
}'
```
- `artifact_zip` points to `target/visionos-builds/<job_id>/artifact.zip`; copy it before `download_ttl_seconds` expires.
- Set `include_logs: false` to omit `log_excerpt` and reduce noise on the client side.


## Startup modes and auth tips

- Switch transports with `--transport {stdio|tcp}` (default `stdio`).
- `--token` wins over `MCP_SHARED_TOKEN`; if neither is set, startup fails with `MCP_TOKEN_REQUIRED` (exit 43).
- Mismatched `[auth].token` yields `AUTH_TOKEN_MISMATCH` (exit 42); TTY stdin/stdout yields `MCP_CLIENT_REQUIRED` (exit 44).
- See [`docs/runbook.md`](./runbook.md) for detailed procedures and troubleshooting.

## Troubleshooting

| Symptom | Resolution |
| --- | --- |
| `CONFIG_MISSING_FIELD auth` | `[auth].token` is missing. Set a 16+ character value. |
| `path_not_allowed` | Add the project’s parent directory to `visionos.allowed_paths`, then restart the server. |
| `sdk_missing` | Install visionOS / Simulator SDK from Xcode > Settings > Platforms. |
| `scheme_not_allowed` | Add the scheme to `visionos.allowed_schemes` and restart the server. |
| `timeout` | Increase `max_build_minutes` or reduce project size/clean build. |
| `artifact_expired` | Call `fetch_build_output` sooner or raise `artifact_ttl_secs`. |

## Logs and telemetry

- `RUST_LOG=debug` enables verbose `tracing`.
- visionOS jobs use the `rmcp_sample::visionos` target and record `job_id`, `status`, and `elapsed_ms` (see [`docs/telemetry.md`](./telemetry.md)).

