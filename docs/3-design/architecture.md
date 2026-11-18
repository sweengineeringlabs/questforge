# QuestForge Architecture

## Overview

QuestForge is organized under **`swartch` Architecture 4.0** as an 11-package Rust workspace (`1.100.0`) paired with a `machineweb` (`mweb`) TypeScript frontend (`web/`). It separates domain contracts (`api`), cohesive domain libraries (`lib`), the CLI binary (`bin`), and external execution targets (Unity, Unreal, Native Rust OpenXR, ADB QA fleets, and Meta HMS/MDMs).

## Stakeholders & Concerns

| Stakeholder | Concerns |
|---|---|
| **XR Developers** | Zero boilerplate, fast local checks (`quest check`), multi-engine parity (`unity`, `unreal`, `rust`) |
| **AI Coding Agents** | Deterministic JSON-RPC 2.0 MCP tools (`quest mcp`) and structured diagnostics |
| **Platform & DevOps** | Provider-agnostic CI (`scripts/ci.sh`), CycloneDX SBOMs, SARIF gates, `swartch` compliance |
| **QA & Release Managers** | Pre-upload VRC scorecards (`quest verify`), ADB lab rollouts, instant rollbacks |
| **Enterprise Ops & Security** | Zero-trust SHA-256 tooling, JWT RBAC, PostgreSQL audit logs, Meta HMS / MDM handoff |

## Component & Layer Diagram

```mermaid
flowchart TD
    subgraph Entrypoints["Entrypoints"]
        CLI["apps/quest (bin: quest CLI)"]
        Web["web/ (mweb + Bun + TypeScript Dashboard)"]
    end

    subgraph Libs["Domain Libraries (package_kind = lib, domain = questforge)"]
        Config["questforge-config (quest_forge.yaml + specs/*_xr.yaml)"]
        Toolchain["questforge-toolchain (Zero-Trust SHA-256 Cache)"]
        Doctor["questforge-doctor (QD-001..QD-009 + XR_RUNTIME_JSON)"]
        Init["questforge-init (quest init, sync, check)"]
        CI["questforge-ci (Build, Rust APK, CycloneDX SBOM)"]
        Verify["questforge-verify (QV-001..QV-006 Scorecards)"]
        MCP["questforge-mcp (12 JSON-RPC 2.0 Tools + LlmProviderSpi)"]
        Ops["questforge-ops (ADB Fleet + Agent + HMS/MDM Push)"]
        Server["questforge-server (REST API, JWT RBAC, PostgreSQL)"]
    end

    subgraph API["Domain Contract (package_kind = api)"]
        ApiCrate["questforge-api (QuestforgeSvc, Builder, Capability, CrateError)"]
    end

    CLI --> Init & Doctor & Toolchain & CI & Verify & MCP & Ops & Server
    Web -->|"REST + X-Quest-Role / Bearer JWT"| Server
    MCP --> Init & Doctor & CI & Verify
    Server --> Ops & Verify
    Init & Doctor & CI & Verify & Ops --> Config
    Config & Toolchain & Doctor & Init & CI & Verify & MCP & Ops & Server --> ApiCrate
```

## End-to-End Data Flow (Code → Horizon OS → Meta HMS)

```mermaid
flowchart LR
    Spec["quest_forge.yaml + specs/*_xr.yaml"] -->|"quest sync"| Gen["AndroidManifest.xml + Bootstrap.gen.* + Scene.gen.*"]
    Gen -->|"quest check (QF-SPEC/SYNC/VRC/OXR/PERF)"| Build["quest ci (Unity / Unreal / Rust OpenXR)"]
    Build -->|"APK + build-metadata.json + sbom.cdx.json"| Verify["quest verify (100/100 Score)"]
    Verify -->|"QA Lab Rollout"| Lab["quest ops deploy / agent (ADB Quest Fleet)"]
    Verify -->|"Store Channel"| Store["ovr-platform-util (ALPHA -> PROD)"]
    Verify -->|"Enterprise OTA"| HMS["quest ops hms-push (Meta HMS / ManageXR / ArborXR)"]
```

## AI / LLM Agent Interaction Flow (`quest agent` ↔ `quest mcp`)

Every autonomous and LLM-driven agent interaction routes strictly through `quest mcp` over JSON-RPC 2.0 (`initialize` → `tools/list` → `tools/call`), with provider credentials and wire formats abstracted behind `LlmProviderSpi` (`GeminiProvider`, `OpenAiProvider`, `AnthropicProvider`) and tailored via `--audience` (`studio`, `enterprise`, `platform`, `qa`, `developer`):

```mermaid
sequenceDiagram
    actor User
    participant Agent as quest agent (LLM ReAct Client)
    participant MCP as quest mcp (JSON-RPC 2.0 Server)
    participant LLM as LlmProviderSpi (Gemini / OpenAI / Anthropic)

    User->>Agent: quest agent --audience studio --provider gemini --auth --prompt "Check, fix, build, and verify orbit-pulse-vr"
    Agent->>MCP: tools/list (JSON-RPC 2.0)
    MCP-->>Agent: 12 QuestForge MCP tool schemas (filtered by audience)
    loop Until LLM returns final answer
        Agent->>LLM: Provider-native request (OpenAI/Gemini Chat Completions or Anthropic /v1/messages)
        LLM-->>Agent: tool_calls / tool_use (e.g. check_source, sync_project, build_project, verify_release)
        Agent->>MCP: tools/call (JSON-RPC 2.0)
        MCP-->>Agent: Structured tool result (JSON)
    end
    Agent-->>User: Final summary + verified 100/100 result
```

- **Auth & Fallback (`quest auth` / `--auth`):** OAuth-first (`gcloud` ADC/OAuth for Gemini, `~/.codex/auth.json` for OpenAI, `~/.claude/.credentials.json` for Anthropic), falling back to environment API keys (`GEMINI_API_KEY` / `GOOGLE_API_KEY`, `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`) or an interactive API-key prompt when OAuth fails.

## Layer Responsibilities

| Layer | Purpose | Rules (`swartch`) |
|---|---|---|
| **`api` (`questforge-api`)** | Domain service, builder, capability traits, and `CrateError` | No dependencies on `lib` crates |
| **`lib` (`questforge-*`)** | Domain logic in private `vo/` & `error/`, re-exported at `lib.rs` | Must depend on `questforge-api`; `compile_fail` doctests enforce module privacy |
| **`bin` (`apps/quest`)** | Thin CLI command dispatch via `clap` | Orchestrates `lib` crates; no business logic |
| **`web/` (`mweb`)** | Declarative `dashboard_*.yaml` → TypeScript UI | Audited by `mweb check --browser` and `csslense` |

## Integration Points

| System | Integration | Purpose |
|---|---|---|
| **Meta Horizon OS OpenXR** | Generated `XR_FB_*` / `XR_EXT_*` extensions + `AndroidManifest.xml` | 72/90/120 Hz, Passthrough, Hand Tracking, Foveation, Spatial Anchors |
| **Meta Horizon Store** | `ovr-platform-util upload-quest-build` (`quest ci`) | Automated store channel promotion |
| **Meta HMS / ManageXR / ArborXR** | HTTPS JSON push (`quest ops hms-push` via `mdm:` in `quest_forge.yaml`) | Enterprise OTA kiosk distribution (`graph.oculus.com`, `managexrapi.com`, `api.xrdm.app`) |
| **PostgreSQL** | `PostgresStore` (`quest serve --database-url`) | Persistent projects, builds, reports, deployments, audit log |
