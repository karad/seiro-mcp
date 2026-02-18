---
layout: default
title: Configuration (config.toml)
lang: en
---

# docs/config.md

## Purpose

Document the `config.toml` keys and validation so misconfiguration is caught during review.

## Location and load order

- Default path: `config.toml` in the repository root
- Env var: `MCP_CONFIG_PATH` overrides the default path
- CLI: `--config` has highest priority; relative paths are resolved to absolute
- The `config` crate parses TOML and blocks startup on missing keys

## Section definitions

| Section | Key | Type | Required | Default | Description |
| --- | --- | --- | --- | --- | --- |
| `[server]` | `host` | `string` | optional | `127.0.0.1` | TCP bind host; default is fine for local testing |
|  | `port` | `u16` | optional (validated) | `8787` | 1024–65535 only; otherwise `CONFIG_INVALID_FIELD` |
| `[auth]` | `token` | `string` | required | - | Shared secret (16–128 chars). Empty/short values yield `CONFIG_MISSING_FIELD`. `--token` or `MCP_SHARED_TOKEN` must match or `AUTH_TOKEN_MISMATCH` (exit 42) is returned. |
| `[visionos]` | `allowed_paths` | `string[]` | required | - | Root directories allowed for builds. Absolute paths only when non-empty. Set to `[]` to disable the allowlist check (any absolute path passes this check). Also used by `validate_sandbox_policy`. |
|  | `allowed_schemes` | `string[]` | required | - | Allowed Xcode scheme names for `build_visionos_app` (1–128 chars, non-empty when non-empty). Set to `[]` to disable the allowlist check (any scheme passes this check). |
|  | `default_destination` | `string` | optional | `platform=visionOS Simulator,name=Apple Vision Pro` | Default `-destination` passed to `xcodebuild`. |
|  | `required_sdks` | `string[]` | optional | `["visionOS", "visionOS Simulator"]` | SDKs that must be installed; empty elements are invalid. |
|  | `xcode_path` | `string` | required | - | Developer dir (`xcode-select -p` equivalent). Absolute path. |
|  | `xcodebuild_path` | `string` | optional | `/usr/bin/xcodebuild` | Full path to `xcodebuild`; can be swapped to a mock in tests. |
|  | `max_build_minutes` | `u16` | optional | `20` | Max duration per visionOS build (1–60). |
|  | `artifact_ttl_secs` | `u32` | optional | `600` | Artifact TTL; expired artifacts are deleted and `fetch_build_output` returns `artifact_expired`. |
|  | `cleanup_schedule_secs` | `u32` | optional | `60` | Interval for TTL cleanup (30–1800 seconds). Too small increases I/O. |

## Example

```toml
[server]
host = "127.0.0.1"
port = 8787

[auth]
token = "change-me-please"

[visionos]
allowed_paths = ["/Users/example/codex/workspaces"]
allowed_schemes = ["VisionApp", "VisionToolbox"]
default_destination = "platform=visionOS Simulator,name=Apple Vision Pro"
required_sdks = ["visionOS", "visionOS Simulator"]
xcode_path = "/Applications/Xcode.app/Contents/Developer"
xcodebuild_path = "/usr/bin/xcodebuild"
max_build_minutes = 20
artifact_ttl_secs = 600
cleanup_schedule_secs = 60
```

List Xcode `scheme` names in `allowed_schemes`. `build_visionos_app` rejects anything outside this allowlist with `scheme_not_allowed`.

> Tip: copy `config.example.toml` first, then edit values to avoid missing keys.

### Disabling allowlists (development only)

You can explicitly disable allowlist checks by setting the lists to empty arrays:

```toml
[visionos]
allowed_paths = []
allowed_schemes = []
```

This makes `build_visionos_app` and `validate_sandbox_policy` skip `path_not_allowed` / `scheme_not_allowed` decisions. This is intended for local development and CI mocks, not for untrusted clients.

### Environment overrides

- `MCP_CONFIG_PATH` takes precedence over the default; `--config` beats both.
- `MCP_SHARED_TOKEN` is compared against `[auth].token`; CLI `--token` wins (blank values are invalid).
- `src/server/config/mod.rs::tests::load_config_from_env_override` covers loading with `tests/fixtures/config_valid.toml`.

## Validation rationale

- `server.port` is validated by `src/server/config/server.rs::validate_port`.
- `auth.token` rejects empty or missing values via `ConfigError::MissingField`.
- `validate_sandbox_policy` uses `[visionos]` for five checks and returns MCP errors on failure:
  1. `allowed_path`: `project_path` is under `allowed_paths` (`path_not_allowed`). If `allowed_paths=[]`, this check is skipped.
  2. `sdk`: all `required_sdks` are visible via `xcodebuild -showsdks` (`sdk_missing`)
  3. `devtools_security`: `DevToolsSecurity -status` reports enabled (`devtools_security_disabled`)
  4. `xcode_license`: `xcodebuild -checkFirstLaunchStatus` succeeds (`xcode_unlicensed`)
  5. `disk_space`: at least 20GB free on the project volume (`disk_insufficient`)
- `validate_sandbox_policy` also returns `diagnostics` (additive, backward-compatible) to explain the evaluation context:
  - `probe_mode`
  - `effective_required_sdks`
  - `detected_sdks_raw` / `detected_sdks_normalized`
  - `effective_developer_dir`
- `inspect_xcode_sdks` is a read-only helper tool that returns the same SDK detection view before running a build.

## Troubleshooting

| Symptom | Resolution |
| --- | --- |
| `CONFIG_MISSING_FIELD auth` | `[auth]` section is missing. Copy the example and set `token` (16–128 chars). |
| `CONFIG_INVALID_FIELD server.port` | Port is outside 1024–65535. Use 8787 unless you have a conflict. |
| `AUTH_TOKEN_MISMATCH` (exit 42) | Ensure `MCP_SHARED_TOKEN` or `--token` matches `[auth].token` (16–128 chars, no spaces). |
| `MCP_TOKEN_REQUIRED` (exit 43) | Token is not set. Provide via env or `--token`. |
| `path_not_allowed` | Add the project’s parent directory to `allowed_paths`. |
| `sdk_missing` | Inspect `details.diagnostics`, run `inspect_xcode_sdks`, then install/fix SDK settings. |
| `devtools_security_disabled` | Run `DevToolsSecurity -enable` and retry. |
| `xcode_unlicensed` | Run `sudo xcodebuild -license` and accept the license. |
| `disk_insufficient` | Free 20GB+ on the same volume as the project. |

## Review checklist

1. After changing `config.toml`, run `cargo test load_config_from_env_override`.
2. Keep config examples in README / Quickstart / Runbook in sync.
3. When specs change, update this table and troubleshooting (including visionOS fields).
4. Cross-check `docs/quickstart.md` sample commands and `specs/002-visionos-mcp/contracts/*.json`; update both if they diverge.
5. Reflect auth errors (exit 42/43) and `MCP_CLIENT_REQUIRED` (exit 44) in Runbook and README troubleshooting.

## Public release hygiene

- Do not commit real tokens: keep `config.toml` local-only and publish examples via `config.example.toml`.
- Avoid personal/local paths in examples. Prefer placeholders like `/Users/<user>/...` or `/home/<user>/...`.
- Treat `[auth].token` as a secret and rotate it if it ever appears in logs, issues, or commit history.
