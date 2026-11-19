# QuestForge Architecture Compliance Checklist

**Audience**: Developers, architects, code reviewers

Use this checklist during code review for any PR that touches QuestForge's architectural boundaries. Every item must pass before merge. This checklist enforces the rules defined in [docs/3-design/architecture.md](../architecture.md).

---

## Table of Contents

- [1. Layer & Package Boundary Compliance (`swartch`)](#1-layer--package-boundary-compliance-swartch)
- [2. Module Privacy & Re-Export Compliance](#2-module-privacy--re-export-compliance)
- [3. AI / MCP Routing & Provider SPI Compliance](#3-ai--mcp-routing--provider-spi-compliance)
- [4. Zero-Trust Toolchain & Cross-Platform / WSL Compliance](#4-zero-trust-toolchain--cross-platform--wsl-compliance)
- [5. Control Plane RBAC & Security Compliance](#5-control-plane-rbac--security-compliance)
- [6. Automated Verification Commands](#6-automated-verification-commands)

---

## 1. Layer & Package Boundary Compliance (`swartch`)

Reference: [docs/3-design/architecture.md](../architecture.md#component--layer-diagram)

- [x] `questforge-api` (`package_kind = api`) has zero dependencies on any `questforge-*` `lib` crate.
- [x] Every domain library crate (`questforge-config`, `questforge-toolchain`, `questforge-doctor`, `questforge-init`, `questforge-ci`, `questforge-verify`, `questforge-mcp`, `questforge-ops`, `questforge-server`) depends on `questforge-api`.
- [x] `apps/quest` (`package_kind = bin`) contains CLI dispatch only (`bin/run.rs`) and delegates business logic to `lib` crates.
- [x] `swartch` reports `0 offences` across all 11 packages.

---

## 2. Module Privacy & Re-Export Compliance

Reference: [docs/3-design/architecture.md](../architecture.md#layer-responsibilities)

- [x] Every `lib` crate keeps `mod error;` and `mod vo;` private in `lib.rs` and re-exports public types at the crate root.
- [x] Every `lib` crate includes `compile_fail` doctests proving internal `error::` and `vo::` paths cannot be imported directly by external consumers.
- [x] `#![deny(unsafe_code)]` is active across workspace crates and no `.unwrap()` / `.expect()` calls appear in non-test production code.

---

## 3. AI / MCP Routing & Provider SPI Compliance

Reference: [docs/3-design/architecture.md](../architecture.md#ai--llm-agent-interaction-flow-quest-agent--quest-mcp)

- [x] All autonomous and LLM-driven agent operations (`quest agent --role`) route strictly through `quest mcp` over JSON-RPC 2.0 (`initialize` → `tools/list` → `tools/call`).
- [x] All 14 MCP tools (`doctor_check`, `sync_project`, `check_source`, `init_project`, `build_project`, `verify_release`, `generate_pipeline`, `analyze_build_failure`, `search_docs`, `fleet_discover`, `publish_mdm`, `setup_toolchain`, `upload_store`, `ovr_store`) return deterministic JSON payloads.
- [x] LLM provider wire formats and OAuth/API-key resolution are isolated behind `LlmProviderSpi` (`GeminiProvider`, `OpenAiProvider`, `AnthropicProvider`).

---

## 4. Zero-Trust Toolchain & Cross-Platform / WSL Compliance

Reference: [docs/3-design/architecture.md](../architecture.md#cross-platform--wsl-interop-architecture)

- [x] `quest setup` verifies SHA-256 checksums before extracting or caching any downloaded binary (`adb`, `cmdline-tools`, `ovr-platform-util`) unless `--allow-unverified` is explicitly set.
- [x] On WSL (`PlatformTarget { os: "wsl", .. }`), native Linux `adb`/`cmdline-tools` are provisioned alongside Windows `ovr-platform-util.exe` (`chmod 0755`), and `quest ops store` / `ovr_store` translate paths via `wslpath -w`.

---

## 5. Control Plane RBAC & Security Compliance

Reference: [docs/3-design/architecture.md](../architecture.md#control-plane--persistence-schema-questforge-server)

- [x] All mutating control plane requests (`POST /api/projects`, `/api/builds`, `/api/verify`, `/api/deployments`) enforce RBAC (`Role::can_write_projects`, `Role::can_deploy`) via `X-Quest-Role` (DevHeader) or HS256 JWT (`Bearer <token>`).
- [x] Every control plane request appends an entry to `questforge_audit_log` (`audit_log`).
- [x] No secrets or tokens (`QUEST_META_APP_SECRET`, `QUESTFORGE_MDM_TOKEN`, `QUESTFORGE_JWT_SECRET`, LLM API keys) are hardcoded or logged.

---

## 6. Automated Verification Commands

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
swartch --vocabulary config/vocabulary.toml crates/questforge-api crates/questforge-config crates/questforge-toolchain crates/questforge-doctor crates/questforge-init crates/questforge-ci crates/questforge-verify crates/questforge-mcp crates/questforge-ops crates/questforge-server apps/quest
```
