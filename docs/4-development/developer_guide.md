# QuestForge Developer Guide

**Audience**: XR Application Developers building Meta Quest apps (`unity`, `unreal`, `rust`) and Platform Contributors developing QuestForge itself

Guide for **XR Application Developers** building Meta Quest apps and **Platform Contributors** developing QuestForge itself.

---

## Table of Contents

- [Part 1: XR Application Developers (`unity` | `unreal` | `rust`)](#part-1-xr-application-developers-unity--unreal--rust)
  - [1. Scaffold & Sync Declarative XR Projects](#1-scaffold--sync-declarative-xr-projects)
  - [2. Static Checks, Toolchain Setup & Diagnostics](#2-static-checks-toolchain-setup--diagnostics)
  - [3. Build, SBOM, Verification & Meta Horizon Store Release](#3-build-sbom-verification--meta-horizon-store-release)
  - [4. Declarative Hand Gestures & Dual-Mode Interaction](#4-declarative-hand-gestures--dual-mode-interaction)
  - [5. Showcase App (`examples/orbit-pulse-vr`) & Enterprise MDM (`quest_forge.yaml` `mdm:`)](#5-showcase-app-examplesorbit-pulse-vr--enterprise-mdm-quest_forgeyaml-mdm)
  - [6. AI Agents & Provider Auth (`quest auth` & `quest agent`)](#6-ai-agents--provider-auth-quest-auth--quest-agent)
- [Part 2: QuestForge Contributors](#part-2-questforge-contributors)
  - [Workspace Structure](#workspace-structure)
  - [Key Conventions](#key-conventions)
  - [See Also](#see-also)

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

### 3. Build, SBOM, Verification & Meta Horizon Store Release

#### Local Build & Compliance Verification
```pwsh
# 1. Build release binaries, package signed APK, and generate CycloneDX SBOM
quest ci --project-dir my-quest-app --output-dir my-quest-app/dist --release-version 1.0.0

# 2. Run static APK compliance verification against Meta VRCs (QV-001..QV-006)
quest verify my-quest-app/dist/my-quest-app-1.0.0.apk --project-dir my-quest-app --format html --out scorecard.html
```

#### Uploading to Meta Horizon Store (Release Channels)
QuestForge embeds full parity with Meta's `ovr-platform-util` (v211.0.0):
* **Credential Auto-Resolution:** The CLI automatically pulls the developer secret from `$env:QUEST_META_APP_SECRET`.
* **App ID Validation & Hierarchy:** The CLI verifies whether `--app-id` (registered in [Meta Horizon Developers](https://developers.meta.com/horizon/manage/)) is provided via:
  1. CLI flag `--app-id <APP_ID>`
  2. Environment variable `$env:QUEST_META_APP_ID`
  3. `mdm.app_id` in `quest_forge.yaml` (via `--project-dir`)
  * **When not set:** The command immediately halts with: `missing Meta Horizon App ID (provide --app-id <APP_ID>, set QUEST_META_APP_ID, or configure mdm.app_id in quest_forge.yaml)`.
  * **When set:** The build is validated and uploaded to the requested channel.
* **Direct Channel Upload (`quest ops store-upload`):**
  ```pwsh
  quest ops store-upload --app-id <APP_ID> --apk my-quest-app/dist/my-quest-app-1.0.0.apk --channel ALPHA --notes "Release 1.0.0"
  ```
* **Integrated CI Build & Store Release (`quest ci`):**
  ```pwsh
  quest ci --project-dir my-quest-app --release-version 1.0.0 --upload-channel ALPHA --app-id <APP_ID>
  ```
* **Granular Store Operations (`quest ops store`):**
  ```pwsh
  # Inspect tool version
  quest ops store --action version

  # Promote build to production
  quest ops store --action set-release-channel-build --app-id <APP_ID> --channel PRODUCTION --build-id <BUILD_ID>
  ```

### 4. Declarative Hand Gestures & Dual-Mode Interaction

QuestForge supports declarative spatial gestures and a stratified hexagonal input layer ([ADR-002](../3-design/adr/002_stratified_input_and_gesture_kinematics.md)). This enables zero-hardware testing: the exact same release APK evaluates hand-wave interactions via OpenXR joint tracking, phone IR proximity sensors, or touch flings.

#### Declaring Gestures in `specs/*_xr.yaml`
```yaml
interactables:
  - id: wave_gesture
    kind: hand_gesture
    gesture: wave
    two_handed: false
    haptic_intensity: 70
    on_activate: OnWaveGestureDetected
```

#### Synchronizing & Implementing Action Handlers
Run `quest sync` to generate the strongly-typed trait and gesture dispatcher:
```pwsh
quest sync --project-dir my-quest-app
```
In your application logic (e.g. `src/orbit_arena_actions.rs` in Rust, or `Assets/Scripts/*` in Unity):
```rust
impl OrbitArenaXrActions for OrbitArenaGame {
    fn on_wave_gesture_detected(&mut self) {
        // Trigger gameplay mechanic (e.g. kinetic shield boost +20%)
        self.shield_charge = self.shield_charge.saturating_add(20).min(100);
        self.score += 50 * self.combo;
    }
}
```

#### Three Evaluation Modes for Developers & Judges
1. **Meta Quest Headset (OpenXR):** Optical camera sensors track 26 joints per hand (`XR_EXT_hand_tracking`). The generated `OpenXrHandWaveDetector` evaluates palm lateral velocity ($v_x > 0.20\text{ m/s}$) and direction alternations at 120 Hz with zero heap allocations.
2. **Physical Android Phone / Tablet:** Deploy the APK to any standard Android device. **Wave your hand physically over the phone's front camera / IR proximity sensor ($< 5\text{ cm}$)** to trigger the wave gesture hands-free.
3. **Headless Android CI / Emulator:** Rapidly swipe across the screen ($|v_x| > 500\text{ px/s}$) or invoke JNI `nativeOnGesture(1)` to run automated regression tests without human intervention.

### 5. Showcase App (`examples/orbit-pulse-vr`) & Enterprise MDM (`quest_forge.yaml` `mdm:`)
Inspect [examples/orbit-pulse-vr](file:///c:/phd-systems/swelabs/metaquest/examples/orbit-pulse-vr/quest_forge.yaml) for a working Native Rust OpenXR project:
```pwsh
cargo test --manifest-path examples/orbit-pulse-vr/Cargo.toml
quest check --project-dir examples/orbit-pulse-vr
```
Optionally add `mdm:` (`meta-hms`, `managexr`, or `arborxr`) in `quest_forge.yaml` so `quest ops hms-push --project-dir my-quest-app --apk dist/app.apk` reads your provider, `app_id`, `channel`, and `device_group` automatically.

### 6. AI Agents & Provider Auth (`quest auth` & `quest agent`)
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

### Workspace Structure
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
  ```

### See Also
- [Architecture](../3-design/architecture.md)
- [Role Workflows](../3-design/workflow/README.md) ([Studio](../3-design/workflow/studio.md) · [Developer](../3-design/workflow/developer.md) · [Platform](../3-design/workflow/platform.md) · [QA](../3-design/workflow/qa.md) · [Enterprise](../3-design/workflow/enterprise.md))
- [Operations Runbook](../6-operations/runbook.md)
