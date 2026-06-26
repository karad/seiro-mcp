# Manual Test Checklist: Cargo Install Distribution Workflow

This checklist verifies the CLI behavior added in `013-cargo-install-distribution`.

Canonical bundled skill source for this release line:

```text
.agents/skills/seiro-mcp-visionos-build-operator/
```

The manual test below still verifies installation into the local Codex skills directory. It does not move the install destination and it does not install the Seiro MCP server binary.

## Preconditions

- Repository root:
  - `/Users/example-user/src/seiro-mcp`
- Rust and Cargo are available.
- On macOS, if commands appear to hang in an integrated terminal (for example VS Code / Codex), rerun the manual test from Terminal.app first. We observed cases where `seiro-mcp --help` was blocked by AppleSystemPolicy evaluation before Rust `main` started, while the same binary completed normally from Terminal.app.

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
CODEX_HOME="$TMP_CODEX_HOME" cargo run -- skill install --dry-run
find "$TMP_CODEX_HOME" -maxdepth 4 -type f
```

Expected:
- JSON `status` is `planned`.
- Both the explicit skill name and omitted skill name forms plan `seiro-mcp-visionos-build-operator`.
- No files are created.

## 4. Install success path

```bash
CODEX_HOME="$TMP_CODEX_HOME" cargo run -- skill install
ls -la "$TMP_CODEX_HOME/.codex/skills/seiro-mcp-visionos-build-operator"
```

Expected:
- JSON `status` is `installed`.
- `SKILL.md` exists under the destination directory.
- Installed files originate from `.agents/skills/seiro-mcp-visionos-build-operator/`.

## 5. Existing-file protection (without force)

```bash
echo "manually-modified" > "$TMP_CODEX_HOME/.codex/skills/seiro-mcp-visionos-build-operator/SKILL.md"
CODEX_HOME="$TMP_CODEX_HOME" cargo run -- skill install
cat "$TMP_CODEX_HOME/.codex/skills/seiro-mcp-visionos-build-operator/SKILL.md"
```

Expected:
- JSON `status` is `skipped_existing`.
- File content remains `manually-modified`.

## 6. Force overwrite path

```bash
CODEX_HOME="$TMP_CODEX_HOME" cargo run -- skill install --force
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

## Example Execution Record

The following example summarizes one successful manual verification run. Paths are intentionally anonymized.

### Environment

- Repository root: `/Users/example-user/src/seiro-mcp`
- Temporary Codex home: `/tmp/example-codex-home`
- Binary under test: `./target/debug/seiro-mcp`
- Execution shell: Terminal.app on macOS

### Recorded Results

1. Dry-run
   ```json
   {
     "destination_dir": "/tmp/example-codex-home/.codex/skills/seiro-mcp-visionos-build-operator",
     "message": "dry-run: no files were modified",
     "skill_name": "seiro-mcp-visionos-build-operator",
     "status": "planned",
     "written_files": [
       "SKILL.md",
       "agents/openai.yaml",
       "assets/seiro-mcp-logo-large.png",
       "assets/seiro-mcp-logo-small.svg"
     ]
   }
   ```
   Verification:
   - `find "/tmp/example-codex-home" -maxdepth 4 -type f` returned no files.

2. Install
   ```json
   {
     "destination_dir": "/tmp/example-codex-home/.codex/skills/seiro-mcp-visionos-build-operator",
     "message": "skill installed",
     "skill_name": "seiro-mcp-visionos-build-operator",
     "status": "installed",
     "written_files": [
       "SKILL.md",
       "agents/openai.yaml",
       "assets/seiro-mcp-logo-large.png",
       "assets/seiro-mcp-logo-small.svg"
     ]
   }
   ```
   Verification:
   - Installed files:
     - `/tmp/example-codex-home/.codex/skills/seiro-mcp-visionos-build-operator/SKILL.md`
     - `/tmp/example-codex-home/.codex/skills/seiro-mcp-visionos-build-operator/agents/openai.yaml`
     - `/tmp/example-codex-home/.codex/skills/seiro-mcp-visionos-build-operator/assets/seiro-mcp-logo-large.png`
     - `/tmp/example-codex-home/.codex/skills/seiro-mcp-visionos-build-operator/assets/seiro-mcp-logo-small.svg`

3. Existing-file protection
   ```json
   {
     "destination_dir": "/tmp/example-codex-home/.codex/skills/seiro-mcp-visionos-build-operator",
     "message": "skill files already exist; re-run with --force to overwrite",
     "skill_name": "seiro-mcp-visionos-build-operator",
     "status": "skipped_existing",
     "written_files": [
       "SKILL.md",
       "agents/openai.yaml",
       "assets/seiro-mcp-logo-large.png",
       "assets/seiro-mcp-logo-small.svg"
     ]
   }
   ```
   Verification:
   - After manually replacing `SKILL.md` with `manually-modified`, the file content remained unchanged without `--force`.

4. Force overwrite
   ```json
   {
     "destination_dir": "/tmp/example-codex-home/.codex/skills/seiro-mcp-visionos-build-operator",
     "message": "skill installed",
     "skill_name": "seiro-mcp-visionos-build-operator",
     "status": "installed",
     "written_files": [
       "SKILL.md",
       "agents/openai.yaml",
       "assets/seiro-mcp-logo-large.png",
       "assets/seiro-mcp-logo-small.svg"
     ]
   }
   ```
   Verification:
   - `SKILL.md` content returned to the bundled skill definition.

5. Remove
   ```json
   {
     "destination_dir": "/tmp/example-codex-home/.codex/skills/seiro-mcp-visionos-build-operator",
     "message": "skill removed",
     "removed_files": [
       "SKILL.md",
       "agents/openai.yaml",
       "assets/seiro-mcp-logo-large.png",
       "assets/seiro-mcp-logo-small.svg"
     ],
     "skill_name": "seiro-mcp-visionos-build-operator",
     "status": "removed"
   }
   ```
   Verification:
   - The target skill directory was removed.
   - A sibling directory such as `/tmp/example-codex-home/.codex/skills/seiro-mcp-keep/` remained intact.

6. Remove non-existent skill
   ```json
   {
     "destination_dir": "/tmp/example-codex-home/.codex/skills/seiro-mcp-visionos-build-operator",
     "message": "skill not found",
     "removed_files": [],
     "skill_name": "seiro-mcp-visionos-build-operator",
     "status": "not_found"
   }
   ```
   Verification:
   - Process exit code was `0`.

## Troubleshooting

- If `cargo run -- skill install ... --dry-run` appears to hang, first run `./target/debug/seiro-mcp --help` from Terminal.app to distinguish a CLI regression from an integrated-terminal execution-policy issue.
- If Terminal.app succeeds but the integrated terminal hangs, treat it as an environment issue outside the Seiro CLI logic and continue the manual test from Terminal.app.
