# QuestForge Developer Guide

Guide for **XR Application Developers** building Meta Quest apps and **Platform Contributors** developing QuestForge itself.

---

## Part 1: XR Application Developers (`unity` | `unreal` | `rust`)

### 1. Scaffold & Sync Declarative XR Projects
```pwsh
# Scaffold Unity, Unreal, or Native Rust OpenXR
quest init my-quest-app --engine rust --package-id com.studio.myquestapp

# Edit quest_forge.yaml (xr_features) and specs/*_xr.yaml, then regenerate
quest sync --project-dir my-quest-app
```
`quest sync` generates VRC-compliant `AndroidManifest.xml`, Horizon OS OpenXR extension bindings (`XR_FB_display_refresh_rate`, `XR_FB_passthrough`, `XR_EXT_hand_tracking`, `XR_FB_foveation`, `XR_FB_spatial_entity`), and strongly-typed action interfaces (`IMainArenaXrActions` / `MainArenaXrActions`).

### 2. Static Checks, Toolchain Setup & Diagnostics

#### Windows (PowerShell)
```pwsh
# Sub-second static gates (QF-SPEC-001, QF-SYNC-001, QF-VRC-001, QF-OXR-001, QF-PERF-001)
quest check --project-dir my-quest-app

# Provision verified Android + Meta Store binaries & check workstation (including XR_RUNTIME_JSON)
quest setup --adb-version latest --ovr-version latest
quest doctor --fix --project-dir my-quest-app
```

#### Linux & WSL (Bash)
```bash
# Sub-second static gates
quest check --project-dir my-quest-app

# Provision native Linux adb/cmdline-tools (+ ovr-platform-util.exe on WSL with chmod 0755)
quest setup --adb-version latest --ovr-version latest
quest doctor --fix --project-dir my-quest-app
```
> **Linux via WSL:** On Windows Subsystem for Linux (WSL), `quest setup --ovr-version latest` automatically detects WSL, installs native Linux `adb`/`cmdline-tools` alongside Meta's Windows `ovr-platform-util.exe` (`0755`), and `quest ops store` / `ovr_store` transparently translate Linux file paths via `wslpath -w` (or reuse an existing Windows host install under `/mnt/c/Users/*/.questforge/`).

### 3. Build, SBOM & Store Verification
```pwsh
quest ci --project-dir my-quest-app --output-dir my-quest-app/dist --release-version 1.0.0
quest verify my-quest-app/dist/my-quest-app-1.0.0.apk --project-dir my-quest-app --format html --out scorecard.html
```

### 4. Showcase App (`examples/orbit-pulse-vr`) & Enterprise MDM (`quest_forge.yaml` `mdm:`)
Inspect [examples/orbit-pulse-vr](file:///c:/phd-systems/swelabs/metaquest/examples/orbit-pulse-vr/quest_forge.yaml) for a working Native Rust OpenXR project:
```pwsh
cargo test --manifest-path examples/orbit-pulse-vr/Cargo.toml
quest check --project-dir examples/orbit-pulse-vr
```
Optionally add `mdm:` (`meta-hms`, `managexr`, or `arborxr`) in `quest_forge.yaml` so `quest ops hms-push --project-dir my-quest-app --apk dist/app.apk` reads your provider, `app_id`, `channel`, and `device_group` automatically.

### 5. AI Agents & Provider Auth (`quest auth` & `quest agent`)
Authenticate or inspect OAuth/API-key status across `gemini`, `openai`, and `anthropic`:
```pwsh
quest auth --status
quest auth --provider gemini
```
Run a role-based XR DevOps agent (`studio`, `enterprise`, `platform`, `qa`, `developer`):
```pwsh
quest agent --role qa --provider gemini --auth --prompt "Verify release readiness and discover connected headsets"
```
If OAuth is unavailable or rejected (`401`/`403`), `quest auth` and `quest agent --auth` fall back to environment API keys or an interactive API-key prompt.

---

## Part 2: QuestForge Contributors

### Workspace Structure (`swartch` Architecture 4.0)
```text
metaquest/
├── apps/quest/              # CLI binary (bin/run.rs)
├── crates/
│   ├── questforge-api/      # Domain contracts (api)
│   └── questforge-*/        # Domain crates (lib: lib.rs, error/, vo/, tests/)
├── examples/orbit-pulse-vr/ # Showcase Native Rust OpenXR app
└── web/                     # machineweb (mweb) + Bun + TypeScript dashboard
```

### Key Conventions
- **Rust `1.100.0`:** Pinned in `rust-toolchain.toml`; `#![deny(unsafe_code)]`; no `.unwrap()`/`.expect()` outside tests.
- **Privacy & Re-exports:** Keep `mod error;` and `mod vo;` private in `lib.rs`, re-export public types at crate root, and preserve `compile_fail` doctests.
- **Verification Gates:**
  ```pwsh
  cargo fmt --all -- --check
  cargo clippy --workspace --all-targets -- -D warnings
  cargo test --workspace
  swartch --vocabulary config/vocabulary.toml crates/questforge-api crates/questforge-config crates/questforge-toolchain crates/questforge-doctor crates/questforge-init crates/questforge-ci crates/questforge-verify crates/questforge-mcp crates/questforge-ops crates/questforge-server apps/quest
  ```

### See Also
- [Architecture](../3-design/architecture.md)
- [Role Workflows](../3-design/workflow/README.md) ([Studio](../3-design/workflow/studio.md) · [Developer](../3-design/workflow/developer.md) · [Platform](../3-design/workflow/platform.md) · [QA](../3-design/workflow/qa.md) · [Enterprise](../3-design/workflow/enterprise.md))
- [Operations Runbook](../6-operations/runbook.md)
