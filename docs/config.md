---
layout: default
title: Configuration (seiro-mcp.toml)
lang: en
---

# docs/config.md

## Purpose

Document the project-local `seiro-mcp.toml` keys and validation so misconfiguration is caught during review.

## Two config files

- Codex config: `~/.codex/config.toml` registers the MCP server command.
- Seiro MCP project config: `seiro-mcp.toml` controls the current project's visionOS behavior.

Generate the Codex-side snippet:

```bash
seiro-mcp config mcp
```

Create the project-local config:

```bash
seiro-mcp config project
```

## Location and load order

- CLI: `--config` has highest priority; relative paths are resolved to absolute.
- Env var: `MCP_CONFIG_PATH` overrides the default path.
- Default path: `seiro-mcp.toml` in the process current directory.
- The `config` crate parses TOML and blocks startup on missing required project keys.

## Minimal project config

```toml
[visionos]
allowed_paths = []
allowed_schemes = []
xcode_path = "/Applications/Xcode.app/Contents/Developer"
```

## Section definitions

| Section | Key | Type | Required | Default | Description |
| --- | --- | --- | --- | --- | --- |
| `[visionos]` | `allowed_paths` | `string[]` | required | - | Root directories allowed for builds. Absolute paths only when non-empty. Set to `[]` to disable the allowlist check. |
|  | `allowed_schemes` | `string[]` | required | - | Allowed Xcode scheme names for `build_visionos_app`. Set to `[]` to disable the allowlist check. |
|  | `default_project_path` | `string` | optional | - | Default path for `inspect_xcode_schemes` when request omits `project_path` and no `.xcodeproj` is found in CWD. Must be an absolute `.xcodeproj` or `.xcworkspace` path. |
|  | `default_destination` | `string` | optional | `platform=visionOS Simulator,name=Apple Vision Pro` | Default `-destination` passed to `xcodebuild`. |
|  | `required_sdks` | `string[]` | optional | `["visionOS", "visionOS Simulator"]` | SDKs that must be installed; empty elements are invalid. |
|  | `xcode_path` | `string` | required | - | Developer dir (`xcode-select -p` equivalent). Absolute path. |
|  | `xcodebuild_path` | `string` | optional | `/usr/bin/xcodebuild` | Full path to `xcodebuild`; can be swapped to a mock in tests. |
|  | `max_build_minutes` | `u16` | optional | `20` | Max duration per visionOS build (1-60). |
|  | `artifact_ttl_secs` | `u32` | optional | `600` | Artifact TTL; expired artifacts are deleted and `fetch_build_output` returns `artifact_expired`. |
|  | `cleanup_schedule_secs` | `u32` | optional | `60` | Interval for TTL cleanup (30-1800 seconds). Too small increases I/O. |

## Full example

```toml
[visionos]
allowed_paths = ["/Users/example/codex/workspaces"]
allowed_schemes = ["VisionApp", "VisionToolbox"]
default_project_path = "/Users/example/codex/workspaces/VisionApp.xcodeproj"
default_destination = "platform=visionOS Simulator,name=Apple Vision Pro"
required_sdks = ["visionOS", "visionOS Simulator"]
xcode_path = "/Applications/Xcode.app/Contents/Developer"
xcodebuild_path = "/usr/bin/xcodebuild"
max_build_minutes = 20
artifact_ttl_secs = 600
cleanup_schedule_secs = 60
```

List Xcode `scheme` names in `allowed_schemes`. `build_visionos_app` rejects anything outside this allowlist with `scheme_not_allowed`.
`inspect_xcode_schemes` resolves `project_path` in this order: request value -> CWD `.xcodeproj` discovery -> `[visionos].default_project_path`.

### Disabling allowlists (development only)

You can explicitly disable allowlist checks by setting the lists to empty arrays:

```toml
[visionos]
allowed_paths = []
allowed_schemes = []
```

This makes `build_visionos_app` and `validate_sandbox_policy` skip `path_not_allowed` / `scheme_not_allowed` decisions. This is intended for local development and CI mocks, not for untrusted clients.

## Codex CLI setup

Use `seiro-mcp config mcp` and paste the output into `~/.codex/config.toml`:

```toml
[mcp_servers.seiro_mcp]
command = "/Users/<user>/.cargo/bin/seiro-mcp"
```

The default local workflow does not require `MCP_SHARED_TOKEN`, `--token`, `--transport`, `working_directory`, or `MCP_CONFIG_PATH`.

## Explicit config path

Use this only when the project config is not named `seiro-mcp.toml` in the working directory.

```bash
MCP_CONFIG_PATH=/absolute/path/to/seiro-mcp.toml seiro-mcp --help
```

For an MCP client, pass `env.MCP_CONFIG_PATH` if the client should use a non-default config path.

## Migrating from project `config.toml`

Older setup instructions used a project-root `config.toml`. Rename or copy only the Seiro MCP project config content to `seiro-mcp.toml`.

```bash
mv config.toml seiro-mcp.toml
```

Do not rename Codex's `~/.codex/config.toml`; that file remains owned by Codex.

## Validation rationale

- `validate_sandbox_policy` uses `[visionos]` for five checks and returns MCP errors on failure:
  1. `allowed_path`: `project_path` is under `allowed_paths` (`path_not_allowed`). If `allowed_paths=[]`, this check is skipped.
  2. `sdk`: all `required_sdks` are visible via `xcodebuild -showsdks` (`sdk_missing`)
  3. `devtools_security`: `DevToolsSecurity -status` reports enabled (`devtools_security_disabled`)
  4. `xcode_license`: `xcodebuild -checkFirstLaunchStatus` succeeds (`xcode_unlicensed`)
  5. `disk_space`: at least 20GB free on the project volume (`disk_insufficient`)
- `validate_sandbox_policy` also returns `diagnostics` to explain the evaluation context:
  - `probe_mode`
  - `effective_required_sdks`
  - `detected_sdks_raw` / `detected_sdks_normalized`
  - `effective_developer_dir`
- `inspect_xcode_sdks` is a read-only helper tool that returns the same SDK detection view before running a build.

## Troubleshooting

| Symptom | Resolution |
| --- | --- |
| Config file not found | Run `seiro-mcp config project` in the project root, or set `MCP_CONFIG_PATH` to an absolute `seiro-mcp.toml` path. |
| `path_not_allowed` | Add the project's parent directory to `allowed_paths`, or use `allowed_paths = []` for local development. |
| `scheme_not_allowed` | Add the Xcode scheme to `allowed_schemes`, or use `allowed_schemes = []` for local development. |
| `sdk_missing` | Inspect `details.diagnostics`, run `inspect_xcode_sdks`, then install/fix SDK settings. |
| `devtools_security_disabled` | Run `DevToolsSecurity -enable` and retry. |
| `xcode_unlicensed` | Run `sudo xcodebuild -license` and accept the license. |
| `disk_insufficient` | Free 20GB+ on the same volume as the project. |

## Review checklist

1. After changing config loading, run the config tests in `src/server/config/mod.rs`.
2. Keep config examples in README / Quickstart / Runbook in sync.
3. When visionOS fields change, update this table and troubleshooting.
4. Cross-check `docs/quickstart.md` sample commands and tool contracts; update both if they diverge.
5. Keep startup errors and `MCP_CLIENT_REQUIRED` (exit 44) aligned with Runbook and README troubleshooting.

## Public release hygiene

- Do not commit local project config with personal paths. Use generated examples or placeholders.
- Avoid personal/local paths in examples. Prefer placeholders like `/Users/<user>/...` or `/home/<user>/...`.
