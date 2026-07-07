---
layout: default
title: README review checklist
lang: en
---

# README review checklist

Use this checklist when README changes affect installation, upgrade, compatibility, or release guidance.

## Installation and upgrade

- The primary install command remains `cargo install seiro-mcp --locked`.
- Upgrade examples use the current package version and match `Cargo.toml`.
- Versioned install examples use the numeric Cargo version without the `v` prefix.
- Skill refresh steps remain separate from MCP server installation.

## Release history

- The README points users to [GitHub Releases](https://github.com/karad/seiro-mcp/releases) as the canonical release history.
- Release notes are expected on GitHub Releases for each published tag.
- README review does not require a repository `CHANGELOG.md`.
- Links to release process details point to `docs/release.md`.

## Public documentation boundaries

- Public README guidance is self-contained and does not depend on private SpecKit files.
- User-facing setup and troubleshooting links point only to public repository files.
- The README remains in English.
