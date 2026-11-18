# QuestForge

> **TLDR:** DevSecOps, declarative XR codegen, VRC verification, MCP agent tooling, and QA fleet management for **Meta Quest / Meta Horizon OS** (`Unity`, `Unreal`, `Native Rust OpenXR`).

## Table of Contents
- [Quick Start](#quick-start)
- [Capabilities](#capabilities)
- [Architecture](#architecture)
- [Documentation](#documentation)

## Quick Start

```pwsh
# 1. Scaffold & sync a Quest project (unity | unreal | rust)
quest init my-quest-app --engine rust
quest sync --project-dir my-quest-app
quest check --project-dir my-quest-app

# 2. Provision verified tools & validate workstation
quest setup --adb-version latest --allow-unverified
quest doctor --fix --project-dir my-quest-app

# 3. Build APK + CycloneDX 1.5 SBOM & verify Meta VRC readiness (0-100)
quest ci --project-dir my-quest-app --output-dir my-quest-app/dist
quest verify my-quest-app/dist/my-quest-app-0.1.0.apk --project-dir my-quest-app

# 4. Deploy to QA lab headsets or push to Meta HMS / MDM
quest ops discover --group qa-lab
quest ops deploy qa-lab my-quest-app/dist/my-quest-app-0.1.0.apk
quest ops hms-push --provider meta-hms --app-id <id> --apk my-quest-app/dist/my-quest-app-0.1.0.apk --group prod
```

## Capabilities

| Component | Kind | Purpose |
|---|---|---|
| `apps/quest` | `bin` | Unified `quest` CLI (`init`, `sync`, `check`, `setup`, `doctor`, `ci`, `verify`, `mcp`, `ops`, `serve`) |
| `crates/questforge-api` | `api` | Domain service/builder/capability contracts |
| `crates/questforge-config` | `lib` | `quest_forge.yaml` (`xr_features`) & `specs/*_xr.yaml` (`XrSceneSpec`) |
| `crates/questforge-init` | `lib` | Scaffolding, declarative XR/OpenXR codegen (`quest sync`), & linter (`quest check`) |
| `crates/questforge-toolchain` | `lib` | Zero-trust SHA-256 binary provisioning & cache (`quest setup`) |
| `crates/questforge-doctor` | `lib` | `QD-001`–`QD-009` diagnostics, `XR_RUNTIME_JSON` check, & `--fix` |
| `crates/questforge-ci` | `lib` | Unity/Unreal/Rust builds, Rust APK packager, CycloneDX SBOM, & CI templates |
| `crates/questforge-verify` | `lib` | Static APK & Meta VRC validator (`QV-001`–`QV-006`) in CLI/JSON/SARIF/HTML/PDF |
| `crates/questforge-mcp` | `lib` | JSON-RPC 2.0 stdio MCP server exposing 12 AI agent tools |
| `crates/questforge-ops` | `lib` | ADB lab fleet manager, polling `agent`, & Meta HMS/ManageXR/ArborXR push |
| `crates/questforge-server` | `lib` | Control Plane REST API, PostgreSQL store, HS256 JWT RBAC, & `/api/audit` |
| `web/` | `mweb` | `machineweb` (`mweb`) + Bun `1.4.2` + TypeScript `7.0.2` dashboard |
| `examples/orbit-pulse-vr` | `example` | Showcase Native Rust OpenXR game (`100/100` verified) |

## Documentation

- [Architecture](docs/3-design/architecture.md) — System layers, OpenXR runtime, and HMS/MDM data flow
- [Workflows (All Actors)](docs/3-design/developer_workflow.md) — End-to-end Mermaid workflows
- [Developer Guide](docs/4-development/developer_guide.md) — App developer & contributor conventions
- [Operations Runbook](docs/6-operations/runbook.md) — Control plane, lab agent, & troubleshooting
- [Product Concept](docs/0-ideation/concept.md) — Audiences, positioning, and engine roadmap
