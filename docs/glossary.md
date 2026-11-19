# Glossary

**Audience**: All QuestForge users, XR developers, platform engineers, QA leads, and contributors

Alphabetized list of terms and acronyms used across **QuestForge**.

---

**ADB (Android Debug Bridge)** - Command-line transport tool from Android `platform-tools` used by `quest doctor` and `quest ops` to discover Quest headsets, install APKs, capture `logcat`, and take screenshots.

**ADR (Architecture Decision Record)** - Document in [docs/3-design/adr/](3-design/adr/README.md) recording a significant architectural decision, its context, and consequences.

**APK (Android Package)** - Signed ARM64 (`arm64-v8a`) application archive deployed to Meta Quest headsets, the Meta Horizon Store, or enterprise MDMs.

**CycloneDX SBOM (Software Bill of Materials)** - Machine-readable supply-chain inventory (`sbom.cdx.json`) emitted by `quest ci`.

**Horizon OS (Meta Horizon OS)** - Meta's Android-based mixed-reality operating system powering Meta Quest 2, Quest Pro, Quest 3, and Quest 3S headsets.

**HMS (Meta Horizon Managed Services)** - Meta's enterprise device and application management platform (`graph.oculus.com`), targeted by `quest ops hms-push` and the `publish_mdm` MCP tool.

**IL2CPP (Intermediate Language To C++)** - Unity's ahead-of-time scripting backend required for 64-bit (`arm64-v8a`) Meta Quest Store submissions.

**LlmProviderSpi** - Outbound service-provider trait in `questforge-mcp` abstracting wire formats and OAuth/API-key authentication across `GeminiProvider`, `OpenAiProvider`, and `AnthropicProvider`.

**MCP (Model Context Protocol)** - JSON-RPC 2.0 protocol over stdio implemented by `quest mcp`, exposing 14 deterministic QuestForge tools to AI coding agents and `quest agent`.

**MDM (Mobile Device Management)** - Enterprise fleet management systems (Meta HMS, ManageXR, ArborXR) that receive verified APKs via `quest ops hms-push`.

**mweb (`machineweb`)** - Declarative component generator and browser verifier powering the `web/` Control Plane dashboard.

**NDK (Android Native Development Kit)** - Toolchain used to compile native C/C++ and Rust (`aarch64-linux-android`) code for Meta Quest.

**OpenXR** - Khronos royalty-free open standard for VR/MR runtime access (`XR_FB_display_refresh_rate`, `XR_FB_passthrough`, `XR_EXT_hand_tracking`, `XR_FB_foveation`, `XR_FB_spatial_entity`).

**ovr-platform-util** - Official Meta Horizon Store command-line utility provisioned by `quest setup --ovr-version latest` and orchestrated 1:1 by `quest ops store` and the `ovr_store` MCP tool.

**QD-001..QD-009 (Quest Doctor Checks)** - Nine workstation and CI diagnostic checks executed by `quest doctor` covering `quest_forge.yaml`, engine, JDK, Android SDK, NDK, OpenXR/Meta XR SDK, ADB/headset, keystore signing, and project structure/vocabulary compliance.

**QF-SPEC / QF-SYNC / QF-VRC / QF-OXR / QF-PERF (Quest Check Gates)** - Five sub-second static source gates run by `quest check` prior to compiling an APK.

**QV-001..QV-006 (Quest Verify Rules)** - Six static binary verification checks executed by `quest verify` on a compiled `.apk` to produce a `0–100` Meta VRC readiness scorecard.

**RBAC (Role-Based Access Control)** - Access enforcement in `quest serve` (`questforge-server`) governing six roles: `administrator`, `platform_engineer`, `developer`, `qa_engineer`, `operations_engineer`, and `auditor`.

**SARIF (Static Analysis Results Interchange Format)** - OASIS standard JSON report format (`--format sarif`) emitted by `quest verify` for CI/CD security tabs.

**VRC (Virtual Reality Checks)** - Meta's technical, performance, manifest, and security compliance requirements for publishing on the Meta Horizon Store.

**WSL (Windows Subsystem for Linux)** - Linux runtime on Windows where QuestForge provisions native Linux `adb`/`cmdline-tools` alongside Windows `ovr-platform-util.exe` and translates paths via `wslpath -w`.

---

## See Also

- [Documentation Hub](README.md)
- [Architecture](3-design/architecture.md)
