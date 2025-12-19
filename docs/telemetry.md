---
layout: default
title: Telemetry / Logging
lang: en
---

# Telemetry / Logging

The visionOS tools emit structured logs with `tracing` so each build job can be tracked by `job_id`. This page summarizes configuration and examples.

## RuntimeModeTelemetry

- Emitted at startup by `src/lib/telemetry.rs::RuntimeModeTelemetry`.
- Fields: `transport` (stdio/tcp), `config_path` (absolute), `host`, `port`, `pending_jobs`, `instructions`.
- Use `RUST_LOG=rmcp_sample=info` or higher to see JSON/text on stderr (Runbook uses this for startup checks).

Example:
```text
INFO rmcp_sample::runtime: starting MCP server
    transport="tcp" config_path="/Users/example/seiro-mcp/config.toml" host="127.0.0.1" port=8787 pending_jobs=0 instructions="load config /Users/.../config.toml ..."
```

## tracing init

- `src/lib/telemetry.rs::init_tracing` is called from `main`; control level with `RUST_LOG` (default `info`).
- Uses `EnvFilter`, so you can set `export RUST_LOG=rmcp_sample=debug,rmcp_sample::visionos=trace` to tune per target.
- MCP clients can pass `RUST_LOG` via their `env` section.

## About JobSpan

- `JobSpan::start(job_id, job_kind)` begins a span under `rmcp_sample::visionos`.
- Close with `finish(status, exit_code)`, always recording `status` (`succeeded` / `failed` …) and `elapsed_ms`.
- Tool usage:
  - `validate_sandbox_policy`: emits a validation span; on failure includes `SandboxPolicyError` code.
  - `build_visionos_app`: wraps xcodebuild; logs `exit_code` and `artifact_path`.
  - `fetch_build_output`: logs artifact retrieval with `job_id` / `include_logs`.

## Log examples

```text
2024-05-12T01:23:45.123Z INFO rmcp_sample::visionos{job_id=1ec5c5c4-6bdc-4f42-9d3c-0d24ebc69212 job_kind="build_visionos_app"}: visionOS job completed
    status="succeeded" exit_code=Some(0) elapsed_ms=5123 artifact_zip="target/visionos-builds/1ec5c5c4-6bdc-4f42-9d3c-0d24ebc69212/artifact.zip"
```

```text
2024-05-12T01:30:02.010Z INFO rmcp_sample::visionos{job_id=de3077d4-1fd5-4a52-89fb-02f6d5c7e5bf job_kind="validate_sandbox_policy"}: visionOS job completed
    status="failed" exit_code=None elapsed_ms=220 code="path_not_allowed"
```

## Operational notes

- All logs go to stderr; Codex CLI shows them under “logs”. For persistence, set `RUST_TRACING_FORMAT=json` and redirect to an external logger.
- `tests/integration/visionos_build.rs` asserts via the job store, not spans; if you change log format, update this doc and README accordingly.
- See [`docs/runbook.md`](./runbook.md) for startup/shutdown exit codes and log checks.

## References

- `src/lib/telemetry.rs` — `init_tracing` and `JobSpan`
- `src/tools/visionos/build.rs` — span start/end points
- `src/tools/visionos/artifacts.rs` — job success/failure and TTL tracking
