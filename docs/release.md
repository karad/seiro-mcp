---
layout: default
title: Release process
lang: en
---

# Release process

This repository does not yet provide an automated release pipeline. Use this document to keep releases reproducible.

## Versioning policy

- Follow SemVer (`MAJOR.MINOR.PATCH`).
- Increment `MAJOR` for breaking changes to MCP tool contracts (tool names, request/response schema, error codes, or structured error fields).
- Increment `MINOR` for backward-compatible additions.
- Increment `PATCH` for fixes that do not change the public contract.

## Pre-release checklist

1. Run the quality gate (fixed order):
   ```bash
   cargo check
   cargo test --all -- --nocapture
   cargo fmt -- --check
   cargo clippy -- -D warnings
   cargo build --release
   ```
2. Run repository checks:
   ```bash
   cargo run -p xtask -- langscan
   cargo run -p xtask -- docs-langscan
   cargo run -p xtask -- check-docs-links
   ```
3. Validate package contents and dry-run publish:
   ```bash
   cargo package --list
   cargo publish --dry-run
   ```
4. Validate docs consistency:
   - README / `docs/quickstart.md` / `docs/runbook.md` / `docs/config.md`
   - `docs/compatibility.md` reflects any contract decision

## Publishing (manual)

- Ensure `Cargo.toml` version, git tag, and release note version are identical.
- Create an annotated git tag (prefer signed tags when available):
  ```bash
  git tag -a vX.Y.Z -m "vX.Y.Z"
  git push origin vX.Y.Z
  ```
- Publish:
  ```bash
  cargo publish
  ```
- Create a GitHub Release for the same tag and include:
  - Change summary
  - Install command: `cargo install seiro-mcp --locked`
  - Compatibility constraints:
    - `seiro-mcp --version` output for the published binary
    - bundled skill target (`seiro-mcp-visionos-build-operator`)
    - skill prefix rule (`seiro-mcp-`)
- Post-publish verification:
  ```bash
  cargo install seiro-mcp --locked --version X.Y.Z
  seiro-mcp --version
  ```
- Attach `target/release/seiro-mcp` if distributing binaries.
