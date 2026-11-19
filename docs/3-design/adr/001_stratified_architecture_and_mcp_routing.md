# ADR-001: Stratified Workspace Architecture, Role-Based MCP Routing, and WSL Store Interop

**Audience**: Architects, technical leads, and QuestForge contributors

## Status

Accepted

## Date

2026-11-18

## Context

QuestForge unifies Meta Quest project scaffolding (`unity`, `unreal`, `rust`), declarative OpenXR code generation (`quest sync`, `quest check`), zero-trust toolchain provisioning (`quest setup`), workstation diagnostics (`quest doctor`), headless CI builds and CycloneDX SBOM generation (`quest ci`), static APK/VRC compliance verification (`quest verify`), AI agent orchestration (`quest mcp`, `quest agent --role`), lab fleet/Store/MDM operations (`quest ops`), and a hosted control plane (`quest serve` + `web/`).

We needed an architecture that:
1. Prevents dependency cycles and leaky abstractions across 11 Rust packages.
2. Ensures AI coding agents and the built-in LLM ReAct loop (`Gemini`, `OpenAI`, `Anthropic`) execute deterministic, auditable operations rather than arbitrary shell commands.
3. Supports Meta Horizon Store operations (`ovr-platform-util`) on both Windows and Linux via WSL despite Meta only publishing Windows and macOS `ovr-platform-util` binaries.

## Decision

1. **Stratified Workspace Architecture:** Split the workspace into one domain contract crate (`questforge-api`, `package_kind = api`), nine domain library crates (`questforge-*`, `package_kind = lib`) with private `error/` and `vo/` modules re-exported at `lib.rs`, and one thin CLI binary (`apps/quest`, `package_kind = bin`).
2. **Strict MCP JSON-RPC 2.0 Agent Boundary:** Route all autonomous and LLM ReAct agent actions (`quest agent --role`) exclusively through `quest mcp` (`14` tools) over JSON-RPC 2.0, with LLM wire protocols isolated behind `LlmProviderSpi`.
3. **Linux via WSL Hybrid Provisioning & Path Translation:** Detect WSL (`PlatformTarget { os: "wsl", .. }`), provision native Linux `adb`/`cmdline-tools` alongside Windows `ovr-platform-util.exe` (`chmod 0755`), share `/mnt/c/Users/*/.questforge/` cache when present, and translate file arguments with `wslpath -w`.

## Comparison

| Aspect | Monolithic CLI + Direct Shell Calls | Stratified Crates + MCP JSON-RPC + WSL Interop (Chosen) |
|---|---|---|
| Layer isolation | Weak; internal types leak across modules | Enforced by crate boundaries and `compile_fail` doctests |
| AI agent safety | Unstructured shell execution | 14 typed JSON-RPC 2.0 MCP tools scoped by `--role` |
| Linux / WSL Store uploads | Blocked (no native Linux `ovr-platform-util`) | Seamless WSL interop via `ovr-platform-util.exe` + `wslpath -w` |

## Rationale

1. **Deterministic Verification:** Every CLI subcommand and MCP tool shares the same domain value objects and error types.
2. **Role-Scoped Autonomy:** Filtering and prioritizing MCP tools per `--role` (`studio`, `enterprise`, `platform`, `qa`, `developer`) keeps prompts focused and enables autonomous QA remediation.
3. **Cross-Platform Parity:** Developers and CI runners on Windows and Linux / WSL use identical `quest setup` and `quest ops store` commands.

## Consequences

### Positive
- Clean layer separation across all 11 Rust packages.
- Full 1:1 `ovr-platform-util` parity across CLI (`quest ops store`) and MCP (`ovr_store`) on Windows and WSL.

### Negative
- Native Linux containers without WSL (`binfmt_misc` Windows PE execution) cannot execute `ovr-platform-util.exe` directly.

### Mitigations
- Provide clear diagnostics and support external `--ovr-bin` overrides or Windows/WSL release runners for Store uploads.

## Related

- [Architecture](../architecture.md)
- [Compliance Checklist](../compliance/compliance_checklist.md)
