---
layout: default
title: Quickstart
lang: en
---

# Quickstart

This repository ships the visionOS build MCP server. Follow these steps to install `seiro-mcp`, register it with Codex, create a project-local `seiro-mcp.toml`, and call the visionOS tools from an MCP client.

## Prerequisites

- macOS 15 Sequoia or later for real visionOS builds
- Xcode 16+ with visionOS / visionOS Simulator SDK
- Rust 1.91.1 (`rustup override set 1.91.1`)
- `cargo`, `git`, `bash`/`zsh`
- An MCP client (Codex CLI or official Inspector)

## Installation

If DevToolsSecurity is disabled, enable it first:

```bash
DevToolsSecurity -status
sudo DevToolsSecurity -enable
```

Install the binary:

```bash
cargo install seiro-mcp --locked
```

For manual testing from a local checkout, install the current working tree instead:

```bash
cd /path/to/seiro-mcp
cargo install --path . --locked --force
```

This builds the local source with the locked dependencies and overwrites the installed `seiro-mcp` binary, usually at `~/.cargo/bin/seiro-mcp`. Use it when you need Codex or Inspector to exercise unmerged local changes through the normal `seiro-mcp` command.

Verify which binary is active:

```bash
which seiro-mcp
seiro-mcp --version
seiro-mcp config mcp
```

To return to the published crates.io build later:

```bash
cargo install seiro-mcp --locked --force
```

## Codex MCP registration

Print the Codex-side MCP registration snippet:

```bash
seiro-mcp config mcp
```

Paste the output into Codex config (`~/.codex/config.toml`):

```toml
[mcp_servers.seiro_mcp]
command = "/Users/<user>/.cargo/bin/seiro-mcp"
```

## Project config

From the target project root:

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

For a non-default config location, pass `MCP_CONFIG_PATH` from your MCP client or use `--config` when launching manually through a client.

## Optional: install bundled skill

```bash
seiro-mcp skill install --dry-run
seiro-mcp skill install
```

- Use `seiro-mcp skill remove seiro-mcp-visionos-build-operator` to remove it.
- The canonical skill source in this repository is `.agents/skills/seiro-mcp-visionos-build-operator/`.
- `seiro-mcp skill install` defaults to `seiro-mcp-visionos-build-operator`. Passing the skill name explicitly is still supported.
- `seiro-mcp skill install` only installs the bundled skill into the local Codex skills directory. It does not install the Seiro MCP server binary and does not configure MCP settings.

Alternative GitHub install path for Codex `skill-installer`:

- `--repo karad/seiro-mcp`
- `--path .agents/skills/seiro-mcp-visionos-build-operator`

## Contributor build checks

If you are contributing to this repository, run:

```bash
cargo fetch
cargo check
cargo test --all -- --nocapture
cargo fmt -- --check
cargo clippy -- -D warnings
cargo build --release
cargo package --list
cargo publish --dry-run
```

Additional repository checks:

```bash
cargo run -p xtask -- langscan
cargo run -p xtask -- docs-langscan
cargo run -p xtask -- check-docs-links
```

## Launch via MCP client

The MCP client must spawn the server as a child process and perform the RMCP handshake over stdio. Running `cargo run` directly without a client will fail with `MCP_CLIENT_REQUIRED`.

Inspector example:

```bash
npx @modelcontextprotocol/inspector seiro-mcp
```

If you are developing from source, replace `seiro-mcp` with `target/release/seiro-mcp`.

## Validate sandbox policy before building

```bash
mcp call validate_sandbox_policy '{
    "project_path": "/Users/<user>/codex/workspaces/vision-app",
    "required_sdks": ["visionOS", "visionOS Simulator"],
    "xcode_path": "/Applications/Xcode.app/Contents/Developer"
}'
```

- If `status: "ok"`, proceed to `build_visionos_app`.
- If `status: "error"` or an MCP error, inspect the code and diagnostics.

Optional SDK inspection:

```bash
mcp call inspect_xcode_sdks '{
    "required_sdks": ["visionOS", "visionOS Simulator"],
    "xcode_path": "/Applications/Xcode.app/Contents/Developer"
}'
```

Optional scheme preflight:

```bash
mcp call inspect_xcode_schemes '{
    "project_path": "/Users/<user>/codex/workspaces/VisionApp/VisionApp.xcodeproj",
    "xcode_path": "/Applications/Xcode.app/Contents/Developer"
}'
```

If `project_path` is omitted, resolution order is:

1. `.xcodeproj` discovered in current working directory
2. `visionos.default_project_path` from `seiro-mcp.toml`

## Build and fetch artifacts

```bash
mcp call build_visionos_app '{
    "project_path": "/Users/<user>/codex/workspaces/VisionApp/VisionApp.xcodeproj",
    "scheme": "VisionApp",
    "destination": "platform=visionOS Simulator,name=Apple Vision Pro",
    "configuration": "debug",
    "extra_args": ["-quiet"]
}'
```

If build fails and returns `job_id`, inspect diagnostics:

```bash
mcp call inspect_build_diagnostics '{
    "job_id": "<UUID returned in build error context>",
    "include_log_excerpt": true,
    "prefer_typecheck": true
}'
```

Fetch artifacts:

```bash
mcp call fetch_build_output '{
    "job_id": "<UUID returned by build_visionos_app>",
    "include_logs": true
}'
```

## Startup mode

- Seiro MCP currently supports the local stdio MCP workflow.
- Token setup is not required for the default local Codex workflow.
- TCP is not part of the supported workflow. If it returns later, it should be redesigned as a separate remote/server mode.

## Troubleshooting

| Symptom | Resolution |
| --- | --- |
| Config file missing | Run `seiro-mcp config project` in the project root, or set `MCP_CONFIG_PATH` to an absolute `seiro-mcp.toml` path. |
| `MCP_CLIENT_REQUIRED` | Launch via an MCP client instead of running `cargo run` directly. |
| `path_not_allowed` | Add the project's parent directory to `visionos.allowed_paths`, or use `allowed_paths = []` for local development. |
| `sdk_missing` | Check `details.diagnostics`, run `inspect_xcode_sdks`, then install/fix SDK settings and retry. |
| `scheme_not_allowed` | Add the scheme to `visionos.allowed_schemes`, or use `allowed_schemes = []` for local development. |
| `artifact_expired` | Call `fetch_build_output` sooner or raise `artifact_ttl_secs`. |

## Logs and telemetry

- `RUST_LOG=debug` enables verbose `tracing`.
- visionOS jobs use the `rmcp_sample::visionos` target and record `job_id`, `status`, and `elapsed_ms` (see [`docs/telemetry.md`](./telemetry.md)).
