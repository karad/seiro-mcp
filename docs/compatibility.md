---
layout: default
title: MCP client compatibility
lang: en
---

## MCP client compatibility

## Support scope

- Supported OS for building/testing the Rust workspace: macOS 15+ and Linux 6.9+.
- visionOS build tools (`validate_sandbox_policy`, `build_visionos_app`, `fetch_build_output`) require macOS + Xcode 16 with visionOS SDK installed.
- On non-macOS environments, the server can compile and tests can run (using mocks), but real visionOS builds are not supported.

## Compatibility policy

This repository treats the MCP surface as a contract:

- Tool names and input/output JSON shapes are stable.
- Error `code` values are stable and are considered part of the public API.
- Structured error metadata is stable:
  - `code`, `remediation`, `retryable`, `sandbox_state`
  - Additional fields may be added, but existing fields should not be removed or renamed.

### What counts as a breaking change

- Renaming/removing a tool or changing required request fields.
- Changing the meaning of an existing error `code`, or replacing a `code` with another one.
- Removing or renaming structured error fields (`code` / `retryable` / `sandbox_state` / `remediation`).

### Supported libraries / protocol

- Uses `rmcp` crate v0.8.5; compatible with Model Context Protocol as of 2024Q2.
- Supported workflow: local stdio MCP server launched as an MCP client child process.
- TCP is not part of the current supported runtime.

## Verified clients

### Model Context Protocol Inspector CLI

```bash
npx @modelcontextprotocol/inspector target/release/seiro-mcp
```

Pass `MCP_CONFIG_PATH` only when the project config is not the current directory's `seiro-mcp.toml`.

### Codex CLI

Generate the registration snippet:

```bash
seiro-mcp config mcp
```

Paste it into `~/.codex/config.toml`:

```toml
[mcp_servers.seiro_mcp]
command = "/Users/<user>/.cargo/bin/seiro-mcp"
```

After restarting Codex CLI, `mcp list` shows the visionOS tools.

## TCP reintroduction policy

TCP should not be restored by re-adding the old shared-token startup flow. If TCP is needed again, design it as a separate remote/server mode with:

- localhost as the default bind address
- explicit opt-in for external bind addresses
- connection-level authentication
- clear exposure guidance
- Inspector-specific verification steps
- compatibility and migration notes

## Compatibility checklist

- [ ] Use `target/release/seiro-mcp` from `cargo build --release`; visionOS tools respond via Inspector over stdio.
- [ ] Codex CLI `command` points to the binary absolute path; tools register successfully without token setup.
- [ ] Missing project config and `MCP_CLIENT_REQUIRED` are reproduced by tests and documented in troubleshooting.

## Known limitations

- Running `cargo run` directly exits immediately because no MCP client is attached (`MCP_CLIENT_REQUIRED` exit 44). Always launch via an MCP client.
- TCP, WebSocket, and other remote transports are not currently supported.
- Without visionOS SDK, `validate_sandbox_policy` returns `sdk_missing`; visionOS tools are unavailable on Linux.
