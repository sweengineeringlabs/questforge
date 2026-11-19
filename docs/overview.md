# QuestForge — Overview & Documentation Index

**QuestForge** puts role-based AI agents (`studio`, `enterprise`, `platform`, `qa`, `developer`) in charge of the Meta Quest and Meta Horizon OS development and release lifecycle.

---

## Documentation Index

### Getting Started & Operations
- [Quick Start Guide](quickstart.md) — Step-by-step installation and agent launch for Windows and Linux / WSL
- [Runbook & Playbook](6-operations/runbook.md) — Control plane, lab operations, Meta Horizon Store uploads, and troubleshooting
- [Developer Guide](4-development/developer_guide.md) — App developer workflows and contributor conventions
- [Showcase App (`orbit-pulse-vr`)](../examples/orbit-pulse-vr/quest_forge.yaml) — Native OpenXR VR showcase with hand tracking, passthrough, spatial anchors, and 120 Hz support

### Architecture & Workflows
- [Architecture](3-design/architecture.md) — System layers, AI/MCP agent flow, WSL interop, and persistence schema
- [Role Workflows Index](3-design/workflow/README.md) — Multi-role ecosystem overview
  - [Studio Workflow (`--role studio`)](3-design/workflow/studio.md) — XR game studio inner-loop and declarative OpenXR codegen
  - [Developer Workflow (`--role developer`)](3-design/workflow/developer.md) — AI-native creator & 14-tool MCP agent workflow
  - [Platform Workflow (`--role platform`)](3-design/workflow/platform.md) — CI/CD orchestration, zero-trust toolchains, and failure diagnostics
  - [QA & Release Workflow (`--role qa`)](3-design/workflow/qa.md) — `100/100` VRC verification, lab headset rollout, and Horizon Store uploads
  - [Enterprise Workflow (`--role enterprise`)](3-design/workflow/enterprise.md) — Control plane governance, RBAC, and Meta HMS / MDM distribution
- [Mobile & Emulator Integration](3-design/mobile-emulator-integration.md) — Device and emulator verification boundary

### Product & Specifications
- [Product Concept & Audiences](0-ideation/concept.md) — Target audiences, positioning, and engine roadmap
- [Platform Domain-Driven Design (DDD)](0-ideation/questForge_platform_ddd.md) — Domain model and bounded contexts
- [Platform SRS](0-ideation/questForge_platform_srs.md) — Software requirements specification
- [Phase 1 MVP Spec](0-ideation/questforge_phase1_mvp_spec.md) — Initial MVP scope and gates
