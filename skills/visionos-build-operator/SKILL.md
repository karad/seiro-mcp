---
name: visionos-build-operator
description: Run seiro-mcp visionOS tools in a safe, explicit sequence (validate_sandbox_policy -> build_visionos_app -> fetch_build_output) when the user explicitly asks to use this skill.
---

# visionos-build-operator

## Purpose

This skill standardizes the operational flow for visionOS builds on Seiro MCP.
It does not replace MCP tools. It orchestrates them in a predictable order.

## When to Use / When Not to Use

Use this skill when:
- The user explicitly asks to use `visionos-build-operator`.
- The user explicitly asks to run the standard visionOS build flow on Seiro MCP.

Do not use this skill when:
- The user did not explicitly request this skill or a skill-based flow.
- The task is unrelated to visionOS build operations.

## Required Inputs

Collect and confirm these inputs before running the flow:
- `project_path` (absolute path, allowed by server policy)
- `scheme`
- Optional `workspace`
- Optional `destination` (defaults to Apple Vision Pro simulator destination)
- Optional `configuration`
- Optional `extra_args`
- Optional `include_logs` for artifact fetch

## Canonical Flow (3 steps)

1. Validate environment and policy first:
   - Call `validate_sandbox_policy` with target project path and required SDKs.
   - If validation fails, stop and return remediation.
   - If diagnostics are needed, inspect `details.diagnostics` (`probe_mode`, `effective_required_sdks`, `detected_sdks_raw`, `detected_sdks_normalized`).

Optional preflight:
- Before build, you MAY call `inspect_xcode_sdks` (read-only) to compare SDK detection context with `validate_sandbox_policy`.

2. Run build:
   - Call `build_visionos_app` with confirmed inputs.
   - On success, capture `job_id`, `artifact_path`, `artifact_sha256`, and `duration_ms`.

3. Fetch artifacts:
   - Call `fetch_build_output` with the `job_id` from step 2.
   - Return artifact metadata and optional logs.

## Error Handling Matrix

| Error code | Where it appears | Action |
| --- | --- | --- |
| `path_not_allowed` | validate/build | Ask user to add project parent path to `visionos.allowed_paths`, restart server, rerun from step 1 |
| `scheme_not_allowed` | build | Ask user to add scheme to `visionos.allowed_schemes`, restart server, rerun from step 2 |
| `sdk_missing` | validate | First inspect `details.diagnostics`; if needed call `inspect_xcode_sdks`; then ask user to install/fix visionOS SDK settings and rerun from step 1 |
| `devtools_security_disabled` | validate | Ask user to run `DevToolsSecurity -enable`, then rerun from step 1 |
| `xcode_unlicensed` | validate | Ask user to accept license (`sudo xcodebuild -license`), then rerun from step 1 |
| `disk_insufficient` | validate | Ask user to free disk space and rerun from step 1 |
| `timeout` | build | Suggest increasing `max_build_minutes` or reducing build load; rerun from step 2 |
| `build_failed` | build | Return log excerpt and failed status; ask whether to inspect logs or retry |
| `artifact_expired` | fetch | Ask user to rerun build to create fresh artifacts, then rerun fetch |
| `job_not_found` | fetch | Verify job ID and rerun build if needed |

## Output Format

Use this concise response template:

- `skill`: `visionos-build-operator`
- `mode`: `explicit-invocation`
- `steps`:
  - `validate_sandbox_policy`: `ok` or `error(<code>)`
  - `build_visionos_app`: `succeeded` or `failed(<code>)`
  - `fetch_build_output`: `succeeded` or `failed(<code>)`
- `job_id`: `<uuid or n/a>`
- `artifact_zip_or_path`: `<path or n/a>`
- `next_action`: `<single concrete next step>`

## Guardrails

- Do not change MCP contracts or tool IDs.
- Do not skip `validate_sandbox_policy` in the standard flow.
- Do not auto-trigger this skill; use only on explicit user request.
- Keep MCP-only usage fully supported and unchanged.
