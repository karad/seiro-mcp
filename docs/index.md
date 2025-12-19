---
layout: default
title: Seiro MCP Overview
lang: en
---

# Seiro MCP Overview

This site hosts the minimal Rust MCP server (rmcp v0.8.5) and the visionOS build tool suite. It summarizes setup steps to call the visionOS tools from Codex / Inspector.

## Quick links

- Quickstart: [docs/quickstart.md](quickstart.md)
- Config and troubleshooting: [docs/config.md](config.md) / [docs/runbook.md](runbook.md)
- Telemetry and logs: [docs/telemetry.md](telemetry.md)
- Compatibility and client setup: [docs/compatibility.md](compatibility.md)
- Releases: [docs/release.md](release.md)
- Repository README: [../README.md](https://github.com/karad/seiro-mcp/blob/main/README.md)
- Open source docs: [LICENSE](https://github.com/karad/seiro-mcp/blob/main/LICENSE) / [CONTRIBUTING.md](https://github.com/karad/seiro-mcp/blob/main/CONTRIBUTING.md) / [CODE_OF_CONDUCT.md](https://github.com/karad/seiro-mcp/blob/main/CODE_OF_CONDUCT.md) / [SECURITY.md](https://github.com/karad/seiro-mcp/blob/main/SECURITY.md)

## Support scope

- Build/test the Rust workspace on macOS 15+ or Linux 6.9+.
- visionOS tools require macOS + Xcode 16 (visionOS / Simulator SDK).
- On non-macOS environments, run tests and docs checks, but do not expect real visionOS builds to work.

## What you can do

- Verify connectivity with `validate_sandbox_policy` / `build_visionos_app` / `fetch_build_output`
- Pre-validate allowed paths, SDKs, and DevToolsSecurity via `validate_sandbox_policy`
- Build a visionOS project and save artifacts with `build_visionos_app`
- Fetch the latest job artifact zip and log excerpt with `fetch_build_output`

## Setup at a glance

1. Get dependencies: `cargo fetch`
2. Create config: copy `config.example.toml`, then adjust `auth.token` and `[visionos]`
3. Validate: `cargo check` → `cargo test --all` → `cargo fmt -- --check` → `cargo clippy -- -D warnings` → `cargo build --release`
4. Launch via MCP client: run `cargo run --quiet -- --transport=stdio` as a child process and align `MCP_SHARED_TOKEN`
5. Call tools: `mcp call validate_sandbox_policy ...` / `mcp call build_visionos_app ...` etc.

See the linked pages for detailed steps and troubleshooting. GitHub Pages uses relative links under `docs/`.
