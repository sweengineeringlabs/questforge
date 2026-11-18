# QuestForge

DevSecOps, release verification, MCP agent tooling, and lab fleet management platform for **Meta Quest** and **Meta Horizon OS** development.

## Quickstart

```pwsh
# 1. Scaffold a Quest-ready Unity or Unreal project
cargo run --bin quest -- init my-quest-app --engine unity

# 2. Synchronize declarative xr_features in quest_forge.yaml into AndroidManifest.xml & bootstrap code
cargo run --bin quest -- sync --project-dir my-quest-app

# 3. Run sub-second static source, VRC manifest, and XR frame-budget checks (QF-SYNC-001..QF-PERF-001)
cargo run --bin quest -- check --project-dir my-quest-app

# 4. Provision Android platform-tools (adb) into ~/.questforge/cache
cargo run --bin quest -- setup --adb-version latest --allow-unverified

# 5. Validate workstation or CI environment (QD-001..QD-009; pass --fix to auto-heal drift)
cargo run --bin quest -- doctor --fix --project-dir my-quest-app

# 6. Run CI build plan, metadata, and CycloneDX 1.5 SBOM generation
cargo run --bin quest -- ci --project-dir my-quest-app --output-dir my-quest-app/dist --dry-run

# 7. Statically verify an APK against Meta Horizon VRC rules (CLI, JSON, SARIF, HTML, PDF)
cargo run --bin quest -- verify path/to/app.apk --format html --out scorecard.html

# 8. Start the MCP server for AI coding agents
cargo run --bin quest -- mcp --list-tools

# 9. Manage lab/device fleets over adb
cargo run --bin quest -- ops discover --group qa-lab

# 10. Start the Control Plane + machineweb (mweb) UI
cargo run --bin quest -- serve --addr 127.0.0.1:8090 --web-root web/dist
```

## Workspace Architecture (`swartch` Architecture 4.0)

| Package | Kind | Purpose |
|---|---|---|
| `crates/questforge-api` | `api` | Domain service/builder/capability contracts and errors |
| `crates/questforge-config` | `lib` | `quest_forge.yaml` schema, validation, and Unity/Unreal version auto-detection |
| `crates/questforge-toolchain` | `lib` | Zero-trust SHA-256 toolchain provisioning & caching (`quest setup`) |
| `crates/questforge-doctor` | `lib` | `QD-001`–`QD-009` workstation/CI diagnostics and `--fix` auto-remediation (`quest doctor`) |
| `crates/questforge-init` | `lib` | Unity/Unreal scaffolding, declarative XR codegen (`quest sync`), and source linting (`quest check`) |
| `crates/questforge-ci` | `lib` | Unity/Unreal build orchestration, metadata, CycloneDX SBOM, and CI templates (`quest ci`) |
| `crates/questforge-verify` | `lib` | Static APK integrity, v1/v2+ signature, `arm64-v8a`, and Meta VRC validator (`quest verify`) |
| `crates/questforge-mcp` | `lib` | JSON-RPC 2.0 stdio MCP server exposing 9 tools (`quest mcp`) |
| `crates/questforge-ops` | `lib` | `adb` fleet discovery, telemetry, deploy/rollback, logcat, screenshot, and lab agent (`quest ops`) |
| `crates/questforge-server` | `lib` | Control plane REST API, swappable `InMemory`/`PostgresStore`, HS256 JWT/RBAC, and audit log (`quest serve`) |
| `apps/quest` | `bin` | Unified `quest` CLI binary |
| `web/` | `mweb` | `machineweb` (`mweb`) + Bun `1.4.2` + TypeScript `7.0.2` control plane dashboard |

## Documentation

- [Product Concept & Target Audiences](docs/0-ideation/concept.md)
- [Phase 1 MVP Specification](docs/0-ideation/questforge_phase1_mvp_spec.md)
- [Platform Vision & Design Document](docs/0-ideation/questForge_platform_ddd.md)
- [Software Requirements Specification](docs/0-ideation/questForge_platform_srs.md)
- [Mobile Emulator Integration Plan](docs/3-design/mobile-emulator-integration.md)
- [Developer Workflow](docs/3-design/developer_workflow.md)
- [Developer Guide](docs/4-development/developer_guide.md)
- [Operations Runbook](docs/6-operations/runbook.md)
