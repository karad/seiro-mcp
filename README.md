<p align="center">
  <img src="./docs/assets/seiro-mcp-logo.png" alt="Seiro MCP Logo" width="240" /><br/>
  <strong>Lightweight visionOS Build MCP for AI Coding Agents</strong>
</p>

[![CI](https://github.com/karad/seiro-mcp/actions/workflows/ci.yml/badge.svg)](https://github.com/karad/seiro-mcp/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Docs](https://img.shields.io/badge/docs-GitHub%20Pages-blue)](https://karad.github.io/seiro-mcp/)

Seiro MCP is a lightweight MCP server focused on visionOS development workflows for AI coding agents. It enables Codex CLI and other MCP clients to safely build, validate, and inspect visionOS projects through a dedicated set of MCP tools.

The goal of Seiro MCP is not to expose every Xcode capability. Instead, it intentionally focuses on the developer workflows that AI coding agents perform most frequently during day-to-day development. Today it provides visionOS build tools together with bundled Codex skill guidance, and over time it will expand with additional developer-focused utilities for spatial computing while remaining focused on development rather than release management.

## Features

- Focused visionOS development workflows for AI coding agents
- Safe build automation through explicit project constraints
- AI-friendly MCP interface for build, diagnostics, and artifacts
- Codex Skill integration for preferred visionOS build operation
- Predictable local development workflow
- Written in Rust

## Why Seiro MCP?

General-purpose Xcode automation is powerful, but AI coding agents usually need a focused subset of development operations: validate the environment, build a project, inspect failures, and retrieve artifacts.

Seiro MCP keeps that interface intentionally small. Smaller MCP tool surfaces are easier for agents to reason about, easier for humans to review, and safer to run in local development environments. The project prioritizes predictable developer workflows over release automation, signing orchestration, or full Xcode replacement.

Lightweight does not mean fewer features. It means exposing the right capabilities for AI-assisted development while deliberately leaving release-oriented workflows, signing orchestration, and broad Xcode automation outside the project's scope.

## Quick Start

```bash
cargo install seiro-mcp --locked
seiro-mcp config mcp
seiro-mcp config project
```

1. Paste the output of `seiro-mcp config mcp` into Codex CLI config (`~/.codex/config.toml`).
2. Run `seiro-mcp config project` from the target project root.
3. Edit `seiro-mcp.toml` to allow the target project paths and schemes.
4. Restart the MCP client and use the visionOS tools.

You are now ready to ask Codex to build your visionOS project with Seiro MCP.

Optional Codex Skill setup:

```bash
seiro-mcp skill install
```

## Installation

### Prerequisites

- Rust 1.91.1 (recommend `rustup override set 1.91.1`)
- Cargo (the `cargo` command must be available)
- Codex CLI
- Any MCP client (e.g., official MCP CLI / Inspector)
- `git`, `bash`/`zsh`

If DevToolsSecurity is disabled, enable it first:

```
$ DevToolsSecurity -status
Developer mode is currently disabled.

$ sudo DevToolsSecurity -enable
```

### 1. Install from crates.io

```bash
cargo install seiro-mcp --locked
```

#### Upgrade to v0.5.1

If you already installed an earlier version, upgrade to `v0.5.1` with:

```bash
cargo install seiro-mcp --locked --force --version 0.5.1
seiro-mcp --version
```

Release history and upgrade notes are published in [GitHub Releases](https://github.com/karad/seiro-mcp/releases).

To refresh bundled skill guidance after upgrading:

```bash
seiro-mcp skill remove seiro-mcp-visionos-build-operator
seiro-mcp skill install
```

### 2. Prepare Codex and project config

(see [`docs/config.md`](docs/config.md) for details)

Generate the Codex-side MCP snippet:

```bash
seiro-mcp config mcp
```

Paste the output into Codex config (`~/.codex/config.toml`):

```toml
[mcp_servers.seiro_mcp]
command = "/Users/<user>/.cargo/bin/seiro-mcp"
```

Then create the project-local Seiro MCP config from the target project root:

```bash
seiro-mcp config project
```

This creates `seiro-mcp.toml`:

```toml
[visionos]
allowed_paths = []
allowed_schemes = []
xcode_path = "/Applications/Xcode.app/Contents/Developer"
```

`MCP_CONFIG_PATH` and `--config` remain available for non-default config locations. When neither is set, Seiro MCP reads `seiro-mcp.toml` from the process current directory.

### 3. Optional: install and run bundled skill

```bash
seiro-mcp skill install --dry-run
seiro-mcp skill install
```

- The bundled skill name uses the `seiro-mcp-` prefix to avoid collisions.
- The bundled skill canonical source is `.agents/skills/seiro-mcp-visionos-build-operator/`, including `SKILL.md`, `agents/openai.yaml`, and icon assets under `.agents/skills/seiro-mcp-visionos-build-operator/assets/`.
- Use `seiro-mcp skill remove seiro-mcp-visionos-build-operator` to roll back.
- `skill remove` returns `not_found` without failing when the skill is already absent.
- Verify compatibility with `seiro-mcp --version` before skill operations.
- For this release line, `seiro-mcp skill install` defaults to `seiro-mcp-visionos-build-operator`. Passing that skill name explicitly is still supported.
- `seiro-mcp skill install` installs the bundled skill into the local Codex skills directory and does not install the Seiro MCP server binary or configure MCP settings.
- Use `seiro-mcp --help`, `seiro-mcp skill --help`, and `seiro-mcp --version` for self-check.
  ```bash
  seiro-mcp --help
  seiro-mcp skill --help
  seiro-mcp skill install --help
  ```

Alternative GitHub install path for Codex `skill-installer`:

- Use Codex `skill-installer` with these arguments when you want to install the skill directly from the public GitHub repository:
  - `--repo karad/seiro-mcp`
  - `--path .agents/skills/seiro-mcp-visionos-build-operator`
- This installs only the Codex skill files. You still need the `seiro-mcp` binary plus MCP server configuration (`seiro-mcp config mcp` and `seiro-mcp config project`).

## Usage

### Using from Codex CLI

Add an entry like the following to Codex CLI config (`~/.codex/config.toml`) to call the visionOS tools:

```toml
[mcp_servers.seiro_mcp]
command = "/Users/<your-username>/.cargo/bin/seiro-mcp"
```

- Codex CLI does not expand `${HOME}`, so use absolute paths and replace `<your-username>`.
- Prefer `seiro-mcp config mcp` to print this snippet with the actual installed binary path.
- By default, Seiro MCP reads `seiro-mcp.toml` from the project current directory.
- Restart Codex CLI and confirm `mcp list` shows the visionOS tools.

### How It Works

#### 1. Launch the server via an MCP client

- The MCP client must spawn the server as a child process and perform the RMCP handshake over stdio. Running `cargo run` directly without a client will fail immediately.
- Example with Inspector:
  ```bash
  npx @modelcontextprotocol/inspector seiro-mcp
  ```
- If you need a non-default config path, pass `MCP_CONFIG_PATH=/absolute/path/to/seiro-mcp.toml`.
- If you are developing from source, build the binary and launch it through an MCP client.

#### 2. Validate sandbox policy before building

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
  - `sdk_missing`: first inspect `details.diagnostics` (`probe_mode`, `effective_required_sdks`, `detected_sdks_raw`, `detected_sdks_normalized`), then install visionOS SDK from Xcode > Settings > Platforms.
  - `devtools_security_disabled`: run `DevToolsSecurity -enable`.
  - `xcode_unlicensed`: run `sudo xcodebuild -license`.
  - `disk_insufficient`: ensure 20GB+ free space for the build.

Optional preflight before build:

```bash
mcp call inspect_xcode_sdks '{
    "required_sdks": ["visionOS", "visionOS Simulator"],
    "xcode_path": "/Applications/Xcode.app/Contents/Developer"
}'
```

- This read-only tool returns `missing_required_sdks` and the same SDK probe context used for sandbox validation.
- Recommended troubleshooting order: `validate_sandbox_policy` diagnostics -> `inspect_xcode_sdks` (optional) -> retry validate/build.

Optional scheme discovery before build:

```bash
mcp call inspect_xcode_schemes '{
    "project_path": "/Users/<user>/codex/workspaces/VisionApp/VisionApp.xcodeproj",
    "xcode_path": "/Applications/Xcode.app/Contents/Developer"
}'
```

- Use this when `project_path` or `scheme` is unknown.
- If `project_path` is omitted, resolution order is:
  1. `.xcodeproj` discovered in current working directory
  2. `visionos.default_project_path` in `seiro-mcp.toml`

#### 3. Start a build with `build_visionos_app`

```bash
mcp call build_visionos_app '{
    "project_path": "/Users/<user>/codex/workspaces/VisionApp/VisionApp.xcodeproj",
    "scheme": "VisionApp",
    "destination": "platform=visionOS Simulator,name=Apple Vision Pro",
    "configuration": "debug",
    "extra_args": ["-quiet"],
    "env_overrides": {"MOCK_XCODEBUILD_BEHAVIOR": "success"}
}'
```

- `project_path` / `workspace` must be absolute paths within `visionos.allowed_paths`.
- `scheme` must be listed in `visionos.allowed_schemes`.
- `configuration` should use lowercase canonical values: `debug` or `release`. For compatibility, `Debug` / `Release` are also accepted.
- Allowed `extra_args`: `-quiet`, `-UseModernBuildSystem=YES`, `-skipPackagePluginValidation`, `-allowProvisioningUpdates`.
- `MOCK_XCODEBUILD_BEHAVIOR` switches the test fixture (`tests/fixtures/visionos/mock-xcodebuild.sh`) among `success` / `fail` / `timeout`.
- On success, returns `job_id`, `artifact_path`, `artifact_sha256`, `log_excerpt`, `duration_ms`; on failure, returns errors such as `build_failed` or `timeout`.
- If multiple simulators match the destination name, `build_visionos_app` returns `destination_ambiguous` with `matched_devices`, `available_destinations`, and a retry-ready `suggested_destination`.

If a build fails, inspect diagnostics without running manual shell commands:

```bash
mcp call inspect_build_diagnostics '{
    "job_id": "<UUID returned in build error context>",
    "include_log_excerpt": true,
    "prefer_typecheck": true
}'
```

- `availability: "available"` returns `primary_location` (`file`, `line`, `column`) from typecheck diagnostics.
- `availability: "unavailable"` falls back to an `xcodebuild_log` summary with notes.

#### 4. Download artifacts with `fetch_build_output`

```bash
mcp call fetch_build_output '{
    "job_id": "<UUID returned by build_visionos_app>",
    "include_logs": true
}'
```

- `artifact_zip` points to `target/visionos-builds/<job_id>/artifact.zip`; copy it before `download_ttl_seconds` expires.
- Set `include_logs: false` to omit `log_excerpt` and reduce noise on the client side.

### Skills Support

Seiro MCP keeps the MCP-only flow unchanged. You can choose either mode:

- MCP-only mode: call `validate_sandbox_policy` / `build_visionos_app` / `inspect_build_diagnostics` (on failure) / `fetch_build_output` directly.
- Skill-assisted mode: use the `seiro-mcp-visionos-build-operator` skill for Xcode / visionOS project workflows so Codex prefers Seiro MCP over direct `xcodebuild` / `swiftc`.
- If `project_path` or `scheme` is missing in skill-assisted mode, run `inspect_xcode_schemes` first as optional preflight.
- When discovering a project locally, remember that `.xcodeproj` and `.xcworkspace` are directory packages. Do not rely on file-only searches such as `rg --files` to decide they are absent.

Skill path in this repository:

- `.agents/skills/seiro-mcp-visionos-build-operator/SKILL.md`

Install from CLI:

```bash
seiro-mcp skill install --dry-run
seiro-mcp skill install
```

Install from GitHub with Codex `skill-installer`:

- `--repo karad/seiro-mcp`
- `--path .agents/skills/seiro-mcp-visionos-build-operator`

Prompt examples:

- `Use seiro-mcp-visionos-build-operator for this visionOS build task.`
- `Please run this using the seiro-mcp-visionos-build-operator skill.`
- `Use Seiro MCP for this Xcode project instead of direct xcodebuild.`

Important:

- Skills provide orchestration guidance.
- MCP provides execution capability.
- In skill-assisted mode, the actual execution remains MCP tool calls with unchanged contracts.
- Installing the skill from GitHub or `seiro-mcp skill install` does not install the Seiro MCP server binary or configure the MCP client connection.
- For Xcode / visionOS project tasks, direct shell `xcodebuild` / `swiftc` should be treated as fallback paths, not the default path.

### Running

- The server must be launched as a child process by an MCP client; running `cargo run` directly will fail with `MCP_CLIENT_REQUIRED` (exit 44).
- Seiro MCP currently supports local stdio MCP startup. TCP mode is not part of the supported local workflow.
- See [`docs/runbook.md`](docs/runbook.md) for the full startup recipe.

### Startup Mode

- `--config` / `MCP_CONFIG_PATH`: `--config` wins; otherwise `MCP_CONFIG_PATH` -> `./seiro-mcp.toml` (relative paths are resolved to absolute).
- Token setup is not required for the default local Codex workflow.
- Exit codes:
  - 44: `MCP_CLIENT_REQUIRED` (stdin/stdout is a TTY; must be launched via MCP client)
- See the Runbook section "Shutdown procedure and exit codes" for details.

### Troubleshooting

- **Config file not found**: run `seiro-mcp config project` in the project root or set an absolute `MCP_CONFIG_PATH`.
- **`MCP_CLIENT_REQUIRED`**: occurs when running `cargo run` directly; always launch via an MCP client (Inspector / Codex, etc.).
- **`seiro-mcp: command not found`**: verify installation and use `seiro-mcp config mcp` to print the Codex MCP snippet.
- **`path_not_allowed`**: add the project parent to `visionos.allowed_paths` and restart.
- **`scheme_not_allowed`**: add the scheme to `visionos.allowed_schemes` and restart.
- **`sdk_missing`**: check `details.diagnostics` first; if `probe_mode` is `env`, verify `VISIONOS_SANDBOX_SDKS`. Then run `inspect_xcode_sdks` and retry after SDK/config fixes.
- **`build_failed`**: use `job_id` from the structured error and call `inspect_build_diagnostics` to identify file/line before retrying.

### References

- visionOS quickstart: [`docs/quickstart.md`](docs/quickstart.md)
- runbook: [`docs/runbook.md`](docs/runbook.md)
- Configuration details: [`docs/config.md`](docs/config.md)

## Motivation

As AI coding agents become more capable, they also need reliable ways to interact with local development environments. Existing solutions often aim to expose broad Xcode functionality, but many autonomous development tasks require only a small, well-defined subset of those capabilities.

Seiro MCP started from a simple idea: provide only the tools that are actually needed for AI-assisted visionOS development, and make those tools reliable, secure, and easy for AI agents to use. Rather than becoming a full-featured Xcode automation server, Seiro MCP is designed to be a focused development companion that helps AI agents build, validate, inspect diagnostics, and support spatial computing projects.

The long-term vision is to grow Seiro MCP into a collection of carefully designed developer tools that improve AI-assisted development for visionOS and spatial computing while preserving its lightweight philosophy.

## Roadmap

Seiro MCP will continue to grow as a focused development toolkit for AI-assisted spatial computing work. Planned directions include:

- Additional visionOS developer tools
- Project analysis utilities
- Additional development workflows for visionOS
- Swift Package support
- Better AI agent workflow guidance
- More spatial computing developer utilities

The roadmap remains aligned with the lightweight philosophy: Seiro MCP should expose the right development tools for AI agents, not become a full-featured Xcode MCP server.

## Contributing

### Directory Layout

```text
src/
  lib/            # shared logic: errors, telemetry, filesystem helpers
  server/         # config + RMCP runtime
  tools/          # visionOS tools
tests/
  integration/    # integration tests (separate crate)
docs/             # configuration, runbook, review checklists
```

### For Contributors (clone + local build)

If you are developing this repository itself, use the clone flow:

```bash
git clone git@github.com:karad/seiro-mcp.git
cd seiro-mcp
cargo fetch
cargo run -p xtask -- langscan
cargo run -p xtask -- docs-langscan
cargo run -p xtask -- check-docs-links
cargo run -p xtask -- preflight
```

If any step fails, fix and rerun.

- On success, `target/release/seiro-mcp` is produced.

### For Maintainers (release readiness)

Before `cargo publish`, run:

```bash
cargo check
cargo test --all -- --nocapture
cargo fmt -- --check
cargo clippy -- -D warnings
cargo build --release
cargo package --list
cargo publish --dry-run
```

`--locked` is recommended for reproducibility, but not mandatory for all environments.

### Tests and Quality Gates

- Preferred: `cargo run -p xtask -- preflight` (runs fetch/check/test/fmt/clippy/build in order).
- Manual: `cargo fetch` -> `cargo check` -> `cargo test --all` -> `cargo fmt -- --check` -> `cargo clippy -- -D warnings` -> `cargo build --release`.
- Unit tests in `src/server/config/mod.rs` cover configuration validation (success and error cases).
- `tests/integration/visionos_build.rs` covers `validate_sandbox_policy`, `build_visionos_app`, `inspect_build_diagnostics`, and `fetch_build_output`, including TTL behavior.

### Open Source

- License: [`LICENSE`](LICENSE)
- Contributing: [`CONTRIBUTING.md`](CONTRIBUTING.md)
- Code of Conduct: [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md)
- Security: [`SECURITY.md`](SECURITY.md)
