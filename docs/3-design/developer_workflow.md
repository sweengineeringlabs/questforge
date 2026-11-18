# QuestForge Developer Workflow

End-to-end workflow for Meta Quest and Meta Horizon OS developers using `quest`, from project bootstrap to store-ready verification and QA lab deployment.

---

## Workflow Overview

```mermaid
flowchart LR
    Init["1. quest init"] --> Setup["2. quest setup"]
    Setup --> Doctor["3. quest doctor"]
    Doctor --> Code["4. Code + quest mcp"]
    Code --> CI["5. quest ci"]
    CI --> Verify["6. quest verify"]
    Verify --> Ops["7. quest ops / quest serve"]
```

---

## Step 1: Bootstrap the Repository (`quest init`)

```pwsh
quest init my-quest-app --engine unity --package-id com.studio.myquestapp
```

Creates a standardized Quest repository:
- `quest_forge.yaml` — project manifest pinning engine version, target Quest devices (`quest2`, `quest3`, `quest3s`, `questpro`), SDK/NDK/JDK versions, and signing env vars
- `ProjectSettings/ProjectVersion.txt` & `Packages/manifest.json` (`com.meta.xr.sdk.all`, `com.unity.xr.openxr`)
- `Assets/Editor/QuestForgeBuild.cs` — headless Unity IL2CPP ARM64 builder (`QuestForge.Build.BuildQuestApk`)
- `config/vocabulary.toml` & `scripts/ci.sh`
- `.gitignore`, `.gitattributes` (Git LFS rules for binary XR assets), `CODEOWNERS`, and `SECURITY.md`

---

## Step 2: Provision Toolchain Binaries (`quest setup`)

```pwsh
quest setup --project-dir my-quest-app --adb-version latest --adb-sha256 <sha256>
```

- Downloads and verifies `adb` (`platform-tools`), Android `cmdline-tools` (`sdkmanager`), and optional `ovr-platform-util` archives against SHA-256 digests under zero-trust policy.
- Extracts atomically into `~/.questforge/cache/<os>-<arch>/<tool>/<version>/` and reuses cached binaries on subsequent runs.

---

## Step 3: Validate Workstation or CI Runner (`quest doctor`)

```pwsh
# Local workstation check
quest doctor --project-dir my-quest-app

# Strict CI pre-flight gate
quest doctor --ci --project-dir my-quest-app --json
```

Runs `QD-001` through `QD-009`:
- `QD-001` Project Config (`quest_forge.yaml` vs engine project files)
- `QD-002` Unity / Unreal Editor + Android IL2CPP support
- `QD-003` JDK (`JAVA_HOME`)
- `QD-004` Android SDK & build-tools
- `QD-005` Android NDK
- `QD-006` Meta XR SDK & OpenXR packages
- `QD-007` `adb` & connected headset (skipped in `--ci`)
- `QD-008` Release signing keystore (enforced in `--ci` / `--strict`)
- `QD-009` `swartch` Architecture 4.0 gate

---

## Step 4: Develop with AI Agent Assistance (`quest mcp`)

While building XR scenes in Unity or Unreal, connect any MCP-compatible coding agent to `quest mcp`:

```pwsh
quest mcp --workspace-root my-quest-app
```

Exposed MCP tools:
- `doctor_check`, `init_project`, `build_project`, `verify_release`, `generate_pipeline`, `analyze_build_failure`, `search_docs`.

---

## Step 5: Build APK, Metadata & SBOM (`quest ci`)

```pwsh
quest ci --project-dir my-quest-app --output-dir my-quest-app/dist --release-version 1.0.0
```

- Invokes Unity (`-batchmode -executeMethod QuestForge.Build.BuildQuestApk`) or Unreal (`RunUAT BuildCookRun`).
- Emits `dist/my-quest-app-1.0.0.apk`, `dist/build-metadata.json` (with APK SHA-256), and CycloneDX 1.5 `dist/sbom.cdx.json`.
- Optionally uploads to a Meta Horizon Store release channel (`--upload-channel ALPHA --app-id <id>`) or emits CI templates (`--emit-pipeline github|azure|jenkins`).

---

## Step 6: Verify Store & VRC Compliance (`quest verify`)

```pwsh
quest verify my-quest-app/dist/my-quest-app-1.0.0.apk --project-dir my-quest-app --format html --out my-quest-app/dist/scorecard.html
```

Audits `QV-001` through `QV-006` before store submission:
- `QV-001` APK archive integrity (`AndroidManifest.xml` + code)
- `QV-002` v1 (`META-INF`) or v2+ (`APK Sig Block 42`) release signature
- `QV-003` 64-bit `arm64-v8a` native ABI compliance
- `QV-004` Meta VRC manifest policy (`debuggable=false`, VR headtracking intent, `package_id` match)
- `QV-005` Sensitive Android permissions audit
- `QV-006` APK size limit gate
- Outputs `0`–`100` scorecards in `cli`, `json`, `sarif`, `html`, or `pdf`.

---

## Step 7: Lab Rollout & Release Governance (`quest ops` & `quest serve`)

```pwsh
# Discover and deploy to QA lab headsets or Android emulators
quest ops discover --group qa-lab
quest ops deploy qa-lab my-quest-app/dist/my-quest-app-1.0.0.apk
quest ops telemetry <serial>
quest ops logcat <serial> --out my-quest-app/dist/device.log
quest ops screenshot <serial> --out my-quest-app/dist/screen.png
quest ops rollback qa-lab

# Or run the polling lab agent + Control Plane dashboard
quest serve --addr 127.0.0.1:8090 --web-root web/dist --database-url postgres://questforge:questforge@127.0.0.1:5432/questforge
quest ops agent --server http://127.0.0.1:8090 --group qa-lab
```
