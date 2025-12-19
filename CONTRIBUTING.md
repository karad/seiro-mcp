# Contributing

Thanks for your interest in contributing!

## Quick start

1. Install Rust 1.91.1 (Edition 2021).
2. Run the local quality gate:
   ```bash
   cargo run -p xtask -- preflight
   ```
3. For docs checks:
   ```bash
   cargo run -p xtask -- docs-langscan
   cargo run -p xtask -- check-docs-links
   ```

## Development workflow

- Prefer small, focused pull requests with clear test output.

## Code style

- Format with rustfmt: `cargo fmt`
- Keep tests green: `cargo test --all`

## Pull requests

- Describe the motivation and the approach.
- Include commands you ran (at least `cargo run -p xtask -- preflight`).
- If you change docs, ensure `cargo run -p xtask -- check-docs-links` succeeds.

## Reporting security issues

Please do not open public issues for security reports.
Email: kazuhiroh+karad@gmail.com (see `SECURITY.md`).

