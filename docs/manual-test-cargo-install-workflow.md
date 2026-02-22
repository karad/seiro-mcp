# Manual Test Checklist: Cargo Install Distribution Workflow

This checklist verifies the CLI behavior added in `013-cargo-install-distribution`.

## Preconditions

- Repository root:
  - `/Users/kazuhirohara/sources/repos/company_works/seiro-mcp`
- Rust and Cargo are available.

## 1. Build sanity check

```bash
cargo check
```

Expected:
- Command succeeds.

## 2. Help and version output

```bash
cargo run -- --help
cargo run -- skill --help
cargo run -- skill install --help
cargo run -- --version
```

Expected:
- Root help includes `skill`, `install`, and `remove` references.
- `skill --help` includes `install` and `remove`, and mentions `--dry-run` behavior.
- `skill install --help` includes `--dry-run`.
- Version format is `seiro-mcp X.Y.Z`.

## 3. Dry-run must be non-mutating

```bash
TMP_CODEX_HOME="$(mktemp -d)"
CODEX_HOME="$TMP_CODEX_HOME" cargo run -- skill install seiro-mcp-visionos-build-operator --dry-run
find "$TMP_CODEX_HOME" -maxdepth 4 -type f
```

Expected:
- JSON `status` is `planned`.
- No files are created.

## 4. Install success path

```bash
CODEX_HOME="$TMP_CODEX_HOME" cargo run -- skill install seiro-mcp-visionos-build-operator
ls -la "$TMP_CODEX_HOME/.codex/skills/seiro-mcp-visionos-build-operator"
```

Expected:
- JSON `status` is `installed`.
- `SKILL.md` exists under the destination directory.

## 5. Existing-file protection (without force)

```bash
echo "manually-modified" > "$TMP_CODEX_HOME/.codex/skills/seiro-mcp-visionos-build-operator/SKILL.md"
CODEX_HOME="$TMP_CODEX_HOME" cargo run -- skill install seiro-mcp-visionos-build-operator
cat "$TMP_CODEX_HOME/.codex/skills/seiro-mcp-visionos-build-operator/SKILL.md"
```

Expected:
- JSON `status` is `skipped_existing`.
- File content remains `manually-modified`.

## 6. Force overwrite path

```bash
CODEX_HOME="$TMP_CODEX_HOME" cargo run -- skill install seiro-mcp-visionos-build-operator --force
head -n 5 "$TMP_CODEX_HOME/.codex/skills/seiro-mcp-visionos-build-operator/SKILL.md"
```

Expected:
- JSON `status` is `installed`.
- File content is replaced by bundled skill content.

## 7. Remove success path

```bash
mkdir -p "$TMP_CODEX_HOME/.codex/skills/seiro-mcp-keep"
echo "keep" > "$TMP_CODEX_HOME/.codex/skills/seiro-mcp-keep/SKILL.md"

CODEX_HOME="$TMP_CODEX_HOME" cargo run -- skill remove seiro-mcp-visionos-build-operator
ls -la "$TMP_CODEX_HOME/.codex/skills"
```

Expected:
- JSON `status` is `removed`.
- `seiro-mcp-visionos-build-operator` is deleted.
- Sibling skill directory (`seiro-mcp-keep`) remains.

## 8. Remove non-existent skill (non-error)

```bash
CODEX_HOME="$TMP_CODEX_HOME" cargo run -- skill remove seiro-mcp-visionos-build-operator
echo $?
```

Expected:
- JSON `status` is `not_found`.
- Exit code is `0`.

## Evidence Notes

Record the following if you want auditable proof:
- Command output snippets (`status`, `message`, and key paths).
- Directory listings before/after install and remove.
- `seiro-mcp --version` output.
