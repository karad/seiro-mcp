# Refactor Responsibility Guidelines

This document defines the responsibility boundaries and dependency direction for the refactor work.

## Dependency direction

- Allowed: `src/lib/` → `src/tools/` → `src/server/`
- Avoid:
  - `src/lib/` importing `src/tools/` or `src/server/`
  - circular dependencies (e.g. `tools` importing `server`)

## Where to put shared helpers

- Put cross-cutting helpers under `src/lib/` (single responsibility per file).
  - Examples: path validation, command assembly, artifact FS helpers, telemetry helpers.
- Tool-specific logic stays under `src/tools/<feature>/...`.
- Runtime wiring and MCP glue stays under `src/server/...`.

## Size targets (review guardrails)

- Target: each Rust file <= 300 LOC.
- Target: top 5 longest Rust files reduced by >= 30% vs baseline.
- Use:
  - `scripts/refactor/loc_baseline.sh` to capture baseline
  - `scripts/refactor/loc_guard.sh` to enforce targets

## Compatibility targets

- Keep these stable across refactors:
  - CLI flags and `--help` output
  - config keys and semantics
  - tool IDs (`build_visionos_app`, `validate_sandbox_policy`, `fetch_build_output`)
  - error codes and remediation messages
  - contracts JSON under `contracts/` and `specs/*/contracts`
- Enforce with:
  - `tests/integration/refactor_contracts.rs` + `tests/fixtures/contracts_sha256.txt`
  - `tests/integration/refactor_behaviour.rs` + `tests/fixtures/refactor/*.json`

