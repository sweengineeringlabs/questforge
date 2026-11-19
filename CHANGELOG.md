# Changelog

All notable changes to **QuestForge** are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.1] - 2026-11-19

### Added
- **Meta Horizon Store (`ovr-platform-util`) 1:1 Parity:** `quest ops store` and MCP `ovr_store` (`upload-quest-build`, `download-quest-build`, `get-release-channel-data`, `set-release-channel-build`, `download-asset-file`, `version`) plus `quest ops store-upload` / MCP `upload_store`.
- **Linux via WSL Interop:** Automatic WSL detection (`PlatformTarget { os: "wsl", .. }`), native Linux `adb`/`cmdline-tools` provisioning alongside Meta's Windows `ovr-platform-util.exe` (`chmod 0755`), Windows host cache discovery (`/mnt/c/Users/*/.questforge/`), and transparent `wslpath -w` path translation.
- **Role-Based Autonomous Agents (`--role`):** Role profiles (`studio`, `enterprise`, `platform`, `qa`, `developer`) for `quest agent`, with autonomous QA remediation and OAuth-to-API-key fallback (`quest auth`).

## [0.1.0] - 2026-11-18

### Added
- **Multi-Engine Scaffolding & Declarative XR Codegen:** `quest init`, `quest sync`, and `quest check` (`QF-SPEC-001`, `QF-SYNC-001`, `QF-VRC-001`, `QF-OXR-001`, `QF-PERF-001`) across Unity (`engine: unity`), Unreal Engine (`engine: unreal`), and Native Rust OpenXR (`engine: rust`).
- **Zero-Trust Toolchain & Workstation Diagnostics:** `quest setup` (SHA-256 verification) and `quest doctor` (`QD-001`..`QD-009` + `--fix`).
- **CI Build, CycloneDX SBOM & Static APK/VRC Verification:** `quest ci` and `quest verify` (`QV-001`..`QV-006`, `0–100` scorecards in CLI, JSON, SARIF, HTML, PDF).
- **MCP Server & Autonomous XR Agent:** `quest mcp` (JSON-RPC 2.0) and `quest agent` with `Gemini`, `OpenAI`, and `Anthropic` (`LlmProviderSpi`) support.
- **Lab Fleet, Enterprise MDM & Hosted Control Plane:** `quest ops` (`discover`, `deploy`, `rollback`, `telemetry`, `logcat`, `screenshot`, `agent`, `hms-push` for Meta HMS / ManageXR / ArborXR) and `quest serve` (PostgreSQL, HS256 JWT RBAC, immutable audit log, and `machineweb` dashboard).
- **Showcase App:** `examples/orbit-pulse-vr` (Native Rust OpenXR with 120 Hz, hand tracking, passthrough, foveation, and spatial anchors).

[0.1.1]: https://github.com/sweengineeringlabs/questforge/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/sweengineeringlabs/questforge/releases/tag/v0.1.0
