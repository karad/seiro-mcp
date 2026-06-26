---
layout: default
title: Runbook
lang: en
---

# Runbook: Start/Stop the MCP Server

## Purpose and scope

- Steps to start/stop `seiro-mcp` from Codex CLI / Inspector and exercise the visionOS tools (`inspect_xcode_schemes` / `validate_sandbox_policy` / `inspect_xcode_sdks` / `build_visionos_app` / `inspect_build_diagnostics` / `fetch_build_output`) within ~30 minutes.
- Target OS: macOS 15 / Linux 6.9+. visionOS builds require Xcode 16 + visionOS SDK.
- Assumes Rust 1.91.1, `cargo`, `bash`/`zsh`.

## Preparation

1. Dependency + build chain:
   ```bash
   cargo run -p xtask -- preflight
   ```
2. Codex MCP registration:
   ```bash
   seiro-mcp config mcp
   ```
   Paste the output into `~/.codex/config.toml`.
3. Project config from the target project root:
   ```bash
   seiro-mcp config project
   ```
   This creates `seiro-mcp.toml`.

## Environment variables

### Optional launch override

- `MCP_CONFIG_PATH`: absolute path to a non-default `seiro-mcp.toml`. Omit this for the normal Codex project-root workflow.

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

- Default config: `seiro-mcp.toml` in the process current directory.
- `--config` or `MCP_CONFIG_PATH`: explicit config path for non-default layouts.
- Transport: local `stdio` only for the supported workflow.
- Token setup is not required for the default local workflow.

### Inspector

```bash
npx @modelcontextprotocol/inspector seiro-mcp
```

If you are developing from source, replace `seiro-mcp` with `target/release/seiro-mcp`.

### Codex CLI example

```toml
[mcp_servers.seiro_mcp]
command = "/Users/<user>/.cargo/bin/seiro-mcp"
```

Use `seiro-mcp config mcp` to print this snippet with the actual binary path.

## TCP status

TCP is not part of the currently supported local workflow. If TCP is reintroduced later, it should be designed as a separate remote/server mode with localhost defaults, connection-level authentication, exposure guidance, and Inspector-specific validation steps.

## Stop flow and exit codes

- `Ctrl+C` (SIGINT) ends with exit code 0.
- Common failure exits:
  - 44: `MCP_CLIENT_REQUIRED` (stdin/stdout is a TTY)
  - Missing config or invalid config: startup exits non-zero and prints structured details to stderr.

## Troubleshooting

| Symptom / code | Resolution |
| --- | --- |
| Config file missing | Run `seiro-mcp config project` in the project root, or set `MCP_CONFIG_PATH` to an absolute `seiro-mcp.toml` path. |
| `MCP_CLIENT_REQUIRED` (44) | You ran `cargo run` directly. Launch via Inspector / Codex as a child process. |
| `seiro-mcp: command not found` | Confirm `cargo install seiro-mcp --locked` completed, then run `seiro-mcp config mcp`. |
| `sdk_missing` | Check `details.diagnostics` from `validate_sandbox_policy`, optionally run `inspect_xcode_sdks`, then install/fix SDK settings and retry. |
| `build_failed` and manual root-cause analysis is slow | Call `inspect_build_diagnostics` with the returned `job_id` to get typecheck-based file/line diagnostics before retrying. |
| `destination_ambiguous` | Re-run `build_visionos_app` with the returned `details.suggested_destination` or choose one entry from `details.available_destinations`. |
| Missing `project_path` / unknown `scheme` | Run `inspect_xcode_schemes` first. If request omits `project_path`, it resolves via current-directory `.xcodeproj` discovery, then `seiro-mcp.toml` `visionos.default_project_path`. |
| `artifact_expired` | Call `fetch_build_output` within TTL; raise `visionos.artifact_ttl_secs` if needed and document the retrieval flow. |
| `seiro-mcp --help` or `skill install --dry-run` hangs only in an integrated terminal | Retry from Terminal.app first. On macOS we observed integrated-terminal launches blocked in AppleSystemPolicy evaluation before Rust `main`, while the same binary completed normally from Terminal.app. |

## Logs and telemetry

- All logs go to stderr. `RUST_LOG=rmcp_sample=info` (or higher) emits runtime telemetry with transport, config path, pending jobs, instructions, and launch args.
- Build job spans are under the `rmcp_sample::visionos` target. Enable JSON logs with `RUST_TRACING_FORMAT=json`.

## Manual verification

1. Run the build chain above.
2. In Inspector mode, confirm `mcp list` shows the visionOS tools.
3. Restart Codex CLI and confirm `mcp describe seiro_mcp` shows the visionOS tools, including `inspect_xcode_sdks`.
4. In the visionOS mock flow, run `inspect_xcode_schemes` (optional preflight) -> `validate_sandbox_policy` -> `inspect_xcode_sdks` (optional) -> `build_visionos_app` -> `inspect_build_diagnostics` (on failure) -> `fetch_build_output` (on success, optionally set `MOCK_XCODEBUILD_BEHAVIOR`).

## Updating installed local skill definitions

When the bundled skill in this repository is updated, refresh local installed copies:

```bash
seiro-mcp skill remove seiro-mcp-visionos-build-operator
seiro-mcp skill install
```

The canonical public skill source is `.agents/skills/seiro-mcp-visionos-build-operator/`.
`seiro-mcp skill install` defaults to `seiro-mcp-visionos-build-operator`; explicit skill-name installation remains supported for compatibility.
If you prefer Codex `skill-installer`, use the GitHub path that maps to that directory:

- `--repo karad/seiro-mcp`
- `--path .agents/skills/seiro-mcp-visionos-build-operator`

This GitHub install path only adds the Codex skill. It does not install the Seiro MCP server binary and it does not configure the MCP client connection.
