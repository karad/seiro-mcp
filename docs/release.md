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

1. Run the quality gate:
   ```bash
   cargo run -p xtask -- preflight
   cargo run -p xtask -- langscan
   cargo run -p xtask -- docs-langscan
   cargo run -p xtask -- check-docs-links
   ```
2. Build the release binary:
   ```bash
   cargo build --release
   ```
3. Validate docs consistency:
   - README / `docs/quickstart.md` / `docs/runbook.md` / `docs/config.md`
   - `docs/compatibility.md` reflects any contract decision

## Publishing (manual)

- Create a Git tag and GitHub Release after confirming the checklist above.
- Attach `target/release/seiro-mcp` if distributing binaries.
