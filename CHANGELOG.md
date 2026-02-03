# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project adheres to Semantic Versioning.

## [0.0.3] - 2026-02-03

### Fixed

- Improved `validate_sandbox_policy` SDK checks to handle Xcode's versioned SDK identifiers (e.g. `xros26.2`, `macosx26.2`) by accepting case-insensitive prefix matches (e.g. `xros` matches `xros26.2`).
- Added stable visionOS SDK aliases (`visionOS` / `visionOS Simulator`, plus `xrOS` variants) to the detected SDK list so configuration and tool calls can use human-friendly names.

## [0.0.2] - 2025-12-23

### Fixed

- update docs readme and quickstart 

## [0.0.1] - 2025-12-16

### Added

- Allow `visionos.allowed_paths=[]` and `visionos.allowed_schemes=[]` to explicitly disable allowlist checks (intended for local development/testing).
- Initial public release of the MCP server and visionOS tool suite.
- `xtask` developer CLI (`preflight`, docs checks, language scan, baselines).
- Documentation under `docs/` (quickstart, runbook, compatibility, release process).
- OSS metadata files (`LICENSE`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SECURITY.md`) and GitHub templates.




