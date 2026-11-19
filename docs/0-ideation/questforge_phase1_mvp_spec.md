# QuestForge — Phase 1 MVP Specification

**Audience**: Platform engineers, CLI contributors, and technical leads

> **TLDR**: Phase 1 MVP specification for QuestForge (`quest setup`, `quest doctor`, `quest init`, and `quest_forge.yaml`), establishing zero-trust toolchain provisioning, workstation diagnostics (`QD-001`..`QD-009`), and standardized repository scaffolding under `swartch` Architecture 4.0.

**Version:** 1.0 (Implemented)  
**Scope:** Phase 1 Foundation (`setup-metavr` / `quest setup`, `quest-doctor` / `quest doctor`, `quest-init` / `quest init`, and shared `quest_forge.yaml`)  
**Toolchain:** Rust `1.100.0`, Bun `1.4.2` (latest stable), `swartch` CLI (Architecture 4.0 auditor, scaffolder, and remediator)  
**Status:** Implemented in `quest` v0.1

---

## Table of Contents

- [1. Phase 1 Objectives & Non-Goals](#1-phase-1-objectives--non-goals)
- [2. Packaging & CI Strategy](#2-packaging--ci-strategy)
- [3. Shared Project Manifest (`quest_forge.yaml`)](#3-shared-project-manifest-quest_forgeyaml)
- [4. Component 1: `setup-metavr` (`quest setup`)](#4-component-1-setup-metavr-cli-provisioner--github-action)
- [5. Component 2: `quest-doctor` (`quest doctor`)](#5-component-2-quest-doctor-environment-validator)
- [6. Component 3: `quest-init` (`quest init`)](#6-component-3-quest-init-project-scaffolder)

---

## 1. Phase 1 Objectives & Non-Goals

### Objectives
Deliver immediate, zero-infrastructure value to Quest/Horizon OS developers and CI pipelines:
1. **Standardized Project Manifest (`quest_forge.yaml`):** A single source of truth for required engine, Android toolchain, Meta XR SDK, and target Quest hardware.
2. **Repeatable Toolchain Setup (`setup-metavr`):** Install and pin Quest-essential CLI tools (`adb` / Android `platform-tools`, Android `cmdline-tools`, and `ovr-platform-util`) locally and in GitHub Actions.
3. **Environment Pre-Flight (`quest-doctor`):** Validate a workstation or CI runner against `quest_forge.yaml` and output actionable remediation steps.
4. **Standardized Scaffolding (`quest-init`):** Bootstrap a Quest-ready Unity or Unreal repository with `quest_forge.yaml`, `.gitignore`, governance files, and starter CI workflows.

### Explicit Non-Goals (Deferred to Phases 2–5)
- No backend API gateway, Kubernetes, PostgreSQL, Redis, S3, or NATS.
- No hosted web dashboard or multi-tenant RBAC/OIDC.
- No enterprise 10,000-device MDM (`questops` is deferred and should target local/lab device farms first).

---

## 2. Packaging & CI Strategy

Instead of maintaining three separate Rust release pipelines in Phase 1, ship:
1. **One Rust (`1.100.0`) CLI binary (`quest`):**
   - `quest init` (also aliasable as `quest-init`)
   - `quest doctor` (also aliasable as `quest-doctor`)
   - `quest setup` (local wrapper around `setup-metavr` logic)
2. **Provider-Agnostic CI (`swartch scaffold --scripts`):**
   - Generate `scripts/ci.sh` and lifecycle scripts via `swartch scaffold --scripts` so CI does not rely on GitHub Actions.
   - `setup-metavr` (or `setup-quest`) remains an optional thin wrapper for teams using GitHub Actions.

---

## 3. Shared Project Manifest (`quest_forge.yaml`)

Every QuestForge-managed repository includes a `quest_forge.yaml` at the repo root. Both `quest doctor` and future `quest-ci` / `quest-verify` phases read this file.

```yaml
version: "1"

project:
  name: my-quest-app
  package_id: com.example.myquestapp
  engine: unity # unity | unreal
  engine_version: "2022.3.42f1" # or auto-detected from ProjectSettings/ProjectVersion.txt

targets:
  devices:
    - quest2
    - quest3
    - quest3s
    - questpro
  min_sdk: 29
  target_sdk: 32
  architecture: arm64-v8a

toolchain:
  jdk: "17"
  android_ndk: "23.1.7779620"
  android_build_tools: "34.0.0"
  metavr_cli:
    adb: "35.0.2"
    ovr_platform_util: "1.99.0"

sdk:
  openxr: true
  meta_xr_sdk: "68.0.0" # checked against Packages/manifest.json (Unity) or Plugins/ (Unreal)

signing:
  keystore_env: QUEST_KEYSTORE_PATH
  alias_env: QUEST_KEY_ALIAS
```

---

## 4. Component 1: `setup-metavr` (CLI Provisioner & GitHub Action)

### Upstream Binaries Managed
"Meta VR CLI" is not a single upstream binary. `setup-metavr` resolves and installs the real tools required for Quest builds and uploads:

| Tool | Purpose | Source / Notes |
|---|---|---|
| `adb` (Android Platform Tools) | Device discovery, APK sideloading, logcat | Official Google `platform-tools` zip per OS |
| `sdkmanager` (Android Command-line Tools) | Headless SDK/NDK/build-tools installation in CI | Official Google `commandlinetools` zip |
| `ovr-platform-util` | Uploading APKs/builds to Meta Horizon Store release channels | Official Meta binary; verify redistribution/EULA and support `cache: false` or user-provided mirror if needed |

### Functional Requirements
- **Cross-Platform:** Linux (`x64`, `arm64`), Windows (`x64`), macOS (`x64`, `arm64`).
- **Version Pinning:** Read versions from action inputs, CLI flags, or `quest_forge.yaml`.
- **Integrity:** Verify SHA-256 checksums for every downloaded archive before extraction.
- **Caching:** Cache extracted binaries by `(os, arch, tool, version)` in `~/.questforge/cache/` locally and via `@actions/cache` in GitHub Actions.
- **PATH & Outputs:** Prepend installed bin directory to `PATH` / `GITHUB_PATH` and emit `adb-path`, `ovr-platform-util-path`, and resolved versions.

### GitHub Action Interface (`action.yml`)
```yaml
inputs:
  config:
    description: "Path to quest_forge.yaml (optional)"
    default: "quest_forge.yaml"
  adb-version:
    description: "Android platform-tools version"
    default: "35.0.2"
  ovr-platform-util-version:
    description: "Meta ovr-platform-util version (omit to skip)"
    default: ""
  cache:
    description: "Enable binary caching"
    default: "true"

outputs:
  bin-dir:
    description: "Directory containing installed binaries"
  adb-version:
    description: "Resolved adb version"
```

---

## 5. Component 2: `quest-doctor` (Environment Validator)

### Purpose
Detect missing, misconfigured, or version-mismatched dependencies *before* a developer spends 20 minutes hitting a Unity/Gradle/NDK build failure.

### Validation Matrix

| Check ID | Target | How It Validates | Pass / Warn / Fail | Remediation Guidance |
|---|---|---|---|---|
| `QD-001` | **Project Config** | Reads `quest_forge.yaml` + `ProjectSettings/ProjectVersion.txt` (Unity) or `*.uproject` (Unreal) | **Fail** if version mismatch between `quest_forge.yaml` and project files | Print exact version mismatch and command to sync |
| `QD-002` | **Unity / Unreal Editor** | Checks OS-standard install paths (Unity Hub `editors.json` / `C:\Program Files\Unity\Hub\Editor\` / `/Applications/Unity/Hub/Editor/`) + **Android Build Support** + **IL2CPP** module | **Fail** if required engine version or Android module is missing | Print Unity Hub CLI command (`unity-hub -- --headless install ...`) |
| `QD-003` | **JDK** | Checks `JAVA_HOME`, Unity embedded JDK, or `java -version` | **Fail** if missing or incompatible major version (e.g., < 11 or != 17) | Suggest setting `JAVA_HOME` or using Unity's embedded JDK |
| `QD-004` | **Android SDK & Build Tools** | Checks `ANDROID_HOME` / `ANDROID_SDK_ROOT` + `platforms/android-<target_sdk>` + `build-tools/<ver>` | **Fail** if missing target platform or build-tools | Print `sdkmanager "platforms;android-32" "build-tools;34.0.0"` |
| `QD-005` | **Android NDK** | Checks `ANDROID_NDK_HOME` or `$ANDROID_HOME/ndk/<ver>` | **Fail** if missing required NDK version | Print `sdkmanager "ndk;23.1.7779620"` |
| `QD-006` | **Meta XR SDK & OpenXR** | Parses `Packages/manifest.json` (Unity) for `com.meta.xr.sdk.all` / `com.unity.xr.openxr` | **Fail** if missing; **Warn** if version < `quest_forge.yaml` | Print UPM package name and expected version |
| `QD-007` | **ADB & Connected Headset** | Runs `adb version` and `adb devices` | **Warn** if no device connected (not fatal in CI); **Fail** if `adb` missing or device `unauthorized` | Prompt to enable Developer Mode & accept RSA USB debugging dialog on headset |
| `QD-008` | **Signing Keystore** | Checks env var / path from `quest_forge.yaml` (`keytool -list`) | **Warn** in local dev; **Fail** with `--strict` / `--ci` flag | Explain how to generate/configure release keystore |
| `QD-009` | **Swartch Architecture Gate** | Checks `swartch` availability, `config/vocabulary.toml` (or `vocabulary.toml`), and runs `swartch <crate-directory>` against Architecture 4.0 invariant rules | **Fail** on rule violations (`[<ERROR_CODE>] <rule_name>`); **Warn** if `swartch` is missing locally (**Fail** in `--ci`) | Run `swartch --fix --dry-run` to preview remediations or `swartch --rule <code > --help` for rule contract |

### CLI & Output Modes
- `quest doctor` — Human-readable checklist (`✓` / `!` / `✗`) with remediation commands.
- `quest doctor --json` — Machine-readable report for CI and future `quest-mcp` consumption.
- `quest doctor --ci` — Strict mode (fails on missing signing config, ignores missing physical USB headset).

---

## 6. Component 3: `quest-init` (Project Scaffolding)

### Purpose
Generate a consistent, Quest-ready repository structure without forcing teams to copy-paste `.gitignore`, Gradle/Unity settings, and GitHub workflows across projects.

### CLI Interface
```bash
quest init my-quest-app --engine unity --unity-version 2022.3.42f1 --package-id com.acme.questapp
```

### Generated Repository Layout
```text
my-quest-app/
├── quest_forge.yaml
├── config/
│   └── vocabulary.toml         # Swartch Architecture 4.0 vocabulary file
├── scripts/                    # Environment & CI lifecycle scripts (`ci.sh` via `swartch scaffold --scripts`)
├── configs/                    # App & environment configs (`swartch scaffold --configs`)
├── .gitignore                  # Tuned for Unity (Library/, Temp/, Logs/) or Unreal (Intermediate/, Saved/, Binaries/)
├── .gitattributes              # Git LFS rules for .fbx, .png, .wav, .apk, .asset
├── .github/
│   ├── CODEOWNERS
│   └── workflows/
│       ├── pr-validate.yml     # Runs setup-metavr + swartch . + quest doctor --ci
│       └── build-apk.yml       # Starter Quest build workflow
├── docs/
│   └── onboarding.md           # Instructions referencing `quest doctor`
├── src/                        # Engine project root (or starter template)
├── README.md
└── SECURITY.md
```

### Functional Requirements
- **Idempotent / Safe:** Refuse to overwrite existing files unless `--force` is passed.
- **Template Support:** Built-in `unity-openxr` and `unreal-openxr` templates, plus `--template <git-url>` for organization-custom templates.
- **Post-Init Hook:** Automatically runs `quest doctor` at the end of `quest init` so the developer immediately sees if their workstation is ready.

---

## 7. Phase 1 Exit Criteria (Definition of Done)

Phase 1 is complete when a new developer or CI runner can:
1. Run `quest init my-app` to generate a valid repository with `quest_forge.yaml`.
2. Run `quest doctor` locally on Windows and macOS and receive accurate pass/fail checks with working remediation commands.
3. Run `setup-metavr` in a Linux/Windows GitHub Actions workflow to install pinned `adb` and `cmdline-tools` with caching and SHA-256 verification.
4. Run `quest doctor --ci --json` in GitHub Actions and fail fast on toolchain drift.
