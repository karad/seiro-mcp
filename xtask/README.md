# xtask

Repository maintenance commands for this workspace.

Run via:

```bash
cargo run -p xtask -- <COMMAND> [ARGS]
```

## Commands

- `preflight`: Runs the local quality gate in order: `cargo fetch` → `cargo check` → `cargo test --all` → `cargo fmt -- --check` → `cargo clippy -- -D warnings` → `cargo build --release`.
- `langscan [PATH]`: Detects Japanese text outside excluded paths (`specs/**`, `.specify/**`, `docs/**`, `.codex/**`, `target/**`) and ignores `AGENTS.md`.
- `docs-langscan [PATH]`: Detects Japanese text under `docs/` (defaults to `docs/`) excluding `docs/ja/**`.
- `check-docs-links [FILES...]`: Validates internal Markdown links and heading anchors under `docs/` (defaults to `docs/*.md` at depth 1). External links are ignored.
- `loc-baseline`: Prints the top 5 longest Rust files under `src/` (line counts).
- `loc-guard [BASELINE]`: Enforces the LOC ceiling (<=300 lines) and the baseline reduction rule (defaults to `specs/008-src-refactor/loc-baseline.txt`).
- `api-baseline [OUT]`: Captures `specs/**/contracts/*.json` SHA-256 and `cargo run -- --help` output into a single baseline file.
- `refactor-check-docs`: Validates required refactor docs exist and are non-empty (Spec 008 helper).

## Examples

```bash
cargo run -p xtask -- preflight
cargo run -p xtask -- langscan
cargo run -p xtask -- docs-langscan
cargo run -p xtask -- check-docs-links
cargo run -p xtask -- loc-baseline
cargo run -p xtask -- loc-guard
cargo run -p xtask -- api-baseline specs/009-oss-release-prep/contracts/api-baseline.txt
```

## Notes

- `scripts/**` are thin wrappers around `xtask` for compatibility; prefer calling `xtask` directly in CI and documentation.
- `.specify/**` is owned by SpecKit and is intentionally not replaced by `xtask`.
