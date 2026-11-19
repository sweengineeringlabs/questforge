# QuestForge Documentation Hub

**Audience**: XR Game Studios, AI-Native XR Creators, Platform & DevOps Engineers, QA & Release Managers, Enterprise Security & Ops Leads, and Contributors

> **Scope**: Central W³H navigation hub for **QuestForge**. For step-by-step installation, start with the [Quick Start Guide](quickstart.md).

---

## Table of Contents

- [WHAT: QuestForge Platform](#what-questforge-platform)
- [WHY: Problems Solved](#why-problems-solved)
- [HOW: Navigate by Role & SDLC Phase](#how-navigate-by-role--sdlc-phase)
  - [Getting Started & Operations](#getting-started--operations)
  - [Architecture & Role Workflows (`3-design/`)](#architecture--role-workflows-3-design)
  - [Product & Specifications (`0-ideation/`)](#product--specifications-0-ideation)
  - [Reference & Glossary](#reference--glossary)

---

## WHAT: QuestForge Platform

**QuestForge** puts autonomous, role-based AI agents (`studio`, `enterprise`, `platform`, `qa`, `developer`) in charge of the **Meta Quest** and **Meta Horizon OS** engineering, compliance, and release lifecycle:
- **Declarative XR Scaffolding & Codegen (`quest init`, `quest sync`, `quest check`)** across Unity (`engine: unity`), Unreal Engine (`engine: unreal`), and Native Rust OpenXR (`engine: rust`).
- **Zero-Trust Toolchain & Workstation Diagnostics (`quest setup`, `quest doctor --fix`)** across Windows and Linux / WSL.
- **Headless CI Builds, CycloneDX SBOMs & `0–100` VRC Verification (`quest ci`, `quest verify`)** prior to store or MDM submission.
- **14-Tool MCP Server & Role-Based AI Agents (`quest mcp`, `quest agent --role`, `quest auth`)** with `Gemini`, `OpenAI`, and `Anthropic`.
- **Lab Fleet, Meta Horizon Store (`ovr-platform-util` 1:1) & Enterprise MDM (`quest ops`, `quest serve`, `web/`)**.

---

## WHY: Problems Solved

| Meta Quest Challenge | QuestForge Solution |
|---|---|
| Hand-editing `AndroidManifest.xml` & OpenXR boilerplate | Declarative `quest_forge.yaml` + `specs/*_xr.yaml` → `quest sync` & `quest check` (`QF-SPEC`, `QF-SYNC`, `QF-VRC`, `QF-OXR`, `QF-PERF`) |
| Workstation & CI SDK/NDK/ADB/OVR drift | Zero-trust SHA-256 `quest setup` (including Linux via WSL interop) + `quest doctor --fix` (`QD-001`..`QD-009`) |
| Surprise Meta Horizon Store VRC rejections | Pre-upload static binary verification (`quest verify` `QV-001`..`QV-006`) with `0–100` scorecards in CLI, JSON, SARIF, HTML, and PDF |
| Disconnected store & enterprise fleet rollouts | `quest ops store` (1:1 `ovr-platform-util`), `quest ops hms-push` (Meta HMS, ManageXR, ArborXR), and `quest serve` Control Plane |

---

## HOW: Navigate by Role & SDLC Phase

### Getting Started & Operations
- [Quick Start Guide](quickstart.md) — Install `quest` on Windows or Linux / WSL, authenticate, and launch your first agent
- [Developer Guide (`4-development/`)](4-development/developer_guide.md) — XR application developer workflows and QuestForge contributor conventions
- [Operations Runbook (`6-operations/`)](6-operations/runbook.md) — Control plane (`quest serve`), lab headset fleet (`quest ops`), Meta Horizon Store (`quest ops store`), and troubleshooting
- [Showcase App (`examples/orbit-pulse-vr`)](../examples/orbit-pulse-vr/quest_forge.yaml) — Native Rust OpenXR VR showcase with hand tracking, passthrough, spatial anchors, and 120 Hz support

### Architecture & Role Workflows (`3-design/`)
- [Architecture](3-design/architecture.md) — System layers, AI/MCP agent flow, Linux via WSL interop, and persistence schema
- [Architecture Compliance Checklist](3-design/compliance/compliance_checklist.md) — Enforceable workspace architecture and security checklist
- [Architecture Decision Records (`3-design/adr/`)](3-design/adr/README.md) — Index of recorded architectural decisions
- [Role Workflows Index (`3-design/workflow/`)](3-design/workflow/README.md) — Multi-role ecosystem overview
  - [Studio Workflow (`--role studio`)](3-design/workflow/studio.md) — XR game studio inner-loop and declarative OpenXR codegen
  - [Developer Workflow (`--role developer`)](3-design/workflow/developer.md) — AI-native creator & 14-tool MCP agent workflow
  - [Platform Workflow (`--role platform`)](3-design/workflow/platform.md) — CI/CD orchestration, zero-trust toolchains, and failure diagnostics
  - [QA & Release Workflow (`--role qa`)](3-design/workflow/qa.md) — `100/100` VRC verification, lab headset rollout, and Horizon Store uploads
  - [Enterprise Workflow (`--role enterprise`)](3-design/workflow/enterprise.md) — Control plane governance, RBAC, and Meta HMS / MDM distribution
- [Mobile & Emulator Integration](3-design/mobile-emulator-integration.md) — Device and emulator verification boundary

### Product & Specifications (`0-ideation/`)
- [Product Concept & Audiences](0-ideation/concept.md) — Target audiences, positioning, and engine roadmap
- [Platform Domain-Driven Design (DDD)](0-ideation/questForge_platform_ddd.md) — Domain model and bounded contexts
- [Platform SRS](0-ideation/questForge_platform_srs.md) — Software requirements specification
- [Phase 1 MVP Spec](0-ideation/questforge_phase1_mvp_spec.md) — Initial MVP scope and gates

### Reference & Glossary
- [Glossary](glossary.md) — Alphabetized definitions of QuestForge, OpenXR, Horizon OS, VRC, and MDM terms
