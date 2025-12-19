---
layout: default
title: Documentation Glossary
lang: en
---

# Documentation Glossary (work in progress)

This table tracks canonical English terms during the docs translation. Add rows as you translate to keep terminology consistent.

| Term | Category (product/command/config key) | Canonical English | Notes |
| --- | --- | --- | --- |
| Seiro MCP | product | Seiro MCP | Product name |
| build_visionos_app | command | build_visionos_app | Keep snake_case as in the tool contract |
| validate_sandbox_policy | command | validate_sandbox_policy | Keep snake_case as in the tool contract |
| fetch_build_output | command | fetch_build_output | Keep snake_case as in the tool contract |
| `[auth].token` | config key | auth.token | 16–128 ASCII characters |
| MCP_SHARED_TOKEN | config key | MCP_SHARED_TOKEN | Env var matching auth.token |
| MCP_CONFIG_PATH | config key | MCP_CONFIG_PATH | Env var for config override |
| visionos.destination | config key | visionos.destination | Simulator destination string |
| visionos.artifact_ttl_secs | config key | visionos.artifact_ttl_secs | TTL for build artifacts |

Add new rows for additional terms (UI labels, headings, error messages) as you translate each page.
