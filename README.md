# Seiro MCP

[![CI](https://github.com/karad/seiro-mcp/actions/workflows/ci.yml/badge.svg)](https://github.com/karad/seiro-mcp/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Docs](https://img.shields.io/badge/docs-GitHub%20Pages-blue)](https://karad.github.io/seiro-mcp/)

**This product is still experimental. Please use it with care.　No warranty is provided.**

Seiro MCP is an MCP server focused on spatial computing development. Today it provides tools to safely run visionOS project builds from Codex CLI, supporting autonomous AI-assisted coding workflows. Over time, it will expand with additional developer-focused utilities.

Detailed start/stop procedures live in [`docs/runbook.md`](docs/runbook.md).

## Motivation

At first, I tried to start autonomous AI-driven coding in my local Codex CLI environment on my Mac, but it didn’t work well in my setup. So I decided to build a simple build tool that provides only the functionality I really needed, and started developing it myself. As I worked on it, I gained deeper insights into autonomous coding and realized new possibilities for this tool. Going forward, I would like to develop various supporting features required for building spatial computing applications as an MCP server.

## Prerequisites

- Rust 1.91.1 (recommend `rustup override set 1.91.1`)
- Cargo (the `cargo` command must be available)
- Codex CLI
- Any MCP client (e.g., official MCP CLI / Inspector)
- `git`, `bash`/`zsh`

## Directory layout

```text
src/
  lib/            # shared logic: errors, telemetry, filesystem helpers
  server/         # config + RMCP runtime
  tools/          # visionOS tools
tests/
  integration/    # integration tests (separate crate)
docs/             # configuration, runbook, review checklists
```


## Installation

is your DevToolsSecurity status is disabled, run `sudo DevToolsSecurity -enable` command.

```
$ DevToolsSecurity -status
Developer mode is currently disabled.

$ sudo DevToolsSecurity -enable
```

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
[mcp_servers.seiro_mcp]
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

## Skills Support (Explicit Invocation)

Seiro MCP keeps the MCP-only flow unchanged. You can choose either mode:

- MCP-only mode: call `validate_sandbox_policy` / `build_visionos_app` / `fetch_build_output` directly.
- Skill-assisted mode: explicitly request the `visionos-build-operator` skill, which orchestrates the same three MCP tools in a fixed sequence.

Skill path in this repository:
- `skills/visionos-build-operator/SKILL.md`

Explicit invocation examples:
- `Use visionos-build-operator for this visionOS build task.`
- `Please run this using the visionos-build-operator skill.`

Important:
- Skills provide orchestration guidance.
- MCP provides execution capability.
- Even in skill-assisted mode, the actual execution remains MCP tool calls with unchanged contracts.

## Running (stdio / tcp)

- The server must be launched as a child process by an MCP client; running `cargo run` directly will fail with `MCP_CLIENT_REQUIRED` (exit 44).
- See [`docs/runbook.md`](docs/runbook.md) for the full stdio/tcp recipes.

## Modes and authentication

- `--transport` / `MCP_CONFIG_PATH` / `--config`: default transport is `stdio`. With `--transport=tcp`, the server listens on `server.host` / `server.port` from config. `--config` wins; otherwise `MCP_CONFIG_PATH` → `./config.toml` (relative paths are resolved to absolute).
- `--token` / `MCP_SHARED_TOKEN`: provide a 16–128 character secret that matches `[auth].token`; the CLI flag takes precedence over the environment variable. Mismatch or missing values fail at startup and print structured errors to stderr.
- Exit codes:
  - 42: `AUTH_TOKEN_MISMATCH` (does not match `[auth].token`)
  - 43: `MCP_TOKEN_REQUIRED` (token missing)
  - 44: `MCP_CLIENT_REQUIRED` (stdin/stdout is a TTY; must be launched via MCP client)
- See the Runbook section “Shutdown procedure and exit codes” for details.

## Tests and quality gates

- Preferred: `cargo run -p xtask -- preflight` (runs fetch/check/test/fmt/clippy/build in order).
- Manual: `cargo fetch` → `cargo check` → `cargo test --all` → `cargo fmt -- --check` → `cargo clippy -- -D warnings` → `cargo build --release`.
- Unit tests in `src/server/config/mod.rs` cover configuration validation (success and error cases).
- `tests/integration/visionos_build.rs` covers `validate_sandbox_policy`, `build_visionos_app`, and `fetch_build_output`, including TTL behavior.


## Troubleshooting

- **Config file not found**: place `config.toml` at repo root or set an absolute `MCP_CONFIG_PATH`.
- **Invalid port**: `server.port` must be 1024–65535; fix before starting via MCP client.
- **Token missing**: startup is blocked if `auth.token` is empty; set a random 16+ character string.
- **`AUTH_TOKEN_MISMATCH` / `MCP_TOKEN_REQUIRED`**: ensure `MCP_SHARED_TOKEN` or `--token` matches `[auth].token` and is 16–128 characters.
- **`MCP_CLIENT_REQUIRED`**: occurs when running `cargo run` directly; always launch via an MCP client (Inspector / Codex, etc.).
- **`path_not_allowed`**: add the project parent to `visionos.allowed_paths` and restart.
- **`scheme_not_allowed`**: add the scheme to `visionos.allowed_schemes` and restart.

## References

- visionOS quickstart: [`docs/quickstart.md`](docs/quickstart.md)
- runbook: [`docs/runbook.md`](docs/runbook.md)
- Configuration details: [`docs/config.md`](docs/config.md)

## Open source

- License: [`LICENSE`](LICENSE)
- Contributing: [`CONTRIBUTING.md`](CONTRIBUTING.md)
- Code of Conduct: [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md)
- Security: [`SECURITY.md`](SECURITY.md)
