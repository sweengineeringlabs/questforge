# QuestForge — Workflows for All Actors

End-to-end workflows and Mermaid architecture/interaction diagrams for every actor in the **QuestForge** ecosystem across **Meta Quest 2, Quest Pro, Quest 3, and Quest 3S (Meta Horizon OS)**.

---

## 1. Multi-Actor Ecosystem Overview

QuestForge connects five core actors across local workstations, AI agents, CI/CD runners, enterprise governance servers, and physical QA headset labs.

```mermaid
flowchart TD
    subgraph Actors["Target Actors"]
        Dev["1. XR Developer (Unity / Unreal)"]
        AIDev["2. AI-Native XR Developer & Agent"]
        DevOps["3. Platform & DevOps Engineer"]
        QA["4. XR QA & Release Manager"]
        Ent["5. Enterprise Security & Ops Lead"]
    end

    subgraph CLI["QuestForge CLI (quest)"]
        Init["quest init"]
        Sync["quest sync (Declarative XR Codegen)"]
        Check["quest check (Static VRC & Frame-Budget Linter)"]
        Setup["quest setup (Zero-Trust SHA-256)"]
        Doctor["quest doctor --fix (QD-001..QD-009)"]
        MCP["quest mcp (JSON-RPC 2.0 Server)"]
        CI["quest ci (Build + CycloneDX SBOM)"]
        Verify["quest verify (QV-001..QV-006)"]
        Ops["quest ops (Fleet & ADB Manager)"]
    end

    subgraph ControlPlane["QuestForge Control Plane"]
        Serve["quest serve (REST API + JWT RBAC + PostgreSQL)"]
        Web["web/ Dashboard (machineweb UI)"]
        Agent["quest ops agent (Lab Polling Daemon)"]
    end

    subgraph Targets["Execution & Distribution Targets"]
        Headsets["Meta Quest Fleet & Emulators"]
        Store["Meta Horizon Store (ALPHA / BETA / RC / PROD)"]
    end

    Dev --> Init
    Dev --> Sync
    Dev --> Check
    Dev --> Setup
    Dev --> Doctor
    Dev --> CI
    Dev --> Verify

    AIDev --> MCP
    MCP --> Sync
    MCP --> Check
    MCP --> Doctor
    MCP --> CI
    MCP --> Verify

    DevOps --> Setup
    DevOps --> Doctor
    DevOps --> CI
    CI --> Store

    QA --> Verify
    QA --> Ops
    QA --> Web
    Ops --> Headsets

    Ent --> Serve
    Ent --> Web
    Serve --> Agent
    Agent --> Headsets
```

---

## 2. Actor 1: XR Developer (Unity & Unreal Studios)

### Goal
Bootstrap a standardized Quest repository, validate the local SDK/NDK/JDK/Editor toolchain, iterate rapidly on scenes, and catch Meta Horizon Store VRC issues locally before opening a pull request.

### Developer Inner-Loop Diagram

```mermaid
flowchart LR
    A["1. quest init"] --> B["2. Edit xr_features in quest_forge.yaml"]
    B --> C["3. quest sync (Auto-Gen Manifest & Bootstrap)"]
    C --> D["4. Write Core Scene Logic"]
    D --> E["5. quest check (QF-SYNC / QF-VRC / QF-PERF)"]
    E -->|"0 Offences"| F["6. quest ci + quest verify"]
    E -->|"Offence"| D
    F --> G["7. quest ops deploy"]
```

### Developer Sequence Diagram

```mermaid
sequenceDiagram
    autonumber
    actor Dev as XR Developer
    participant CLI as quest CLI
    participant Cache as ~/.questforge/cache
    participant Engine as Unity / Unreal Editor
    participant Quest as Local Quest Headset

    Dev->>CLI: quest init my-quest-app --engine unity --package-id com.studio.app
    CLI-->>Dev: Scaffolds quest_forge.yaml, QuestForgeBuild.cs, scripts/ci.sh, Git LFS
    Dev->>CLI: quest setup --project-dir my-quest-app --adb-version latest --adb-sha256 <hash>
    CLI->>Cache: Downloads & verifies SHA-256 of platform-tools (adb)
    Dev->>CLI: quest doctor --project-dir my-quest-app
    CLI-->>Dev: Runs QD-001..QD-009 (Config, Editor, JDK, SDK, NDK, Meta XR, ADB, Keystore, swartch)
    Dev->>CLI: quest ci --project-dir my-quest-app --output-dir dist --release-version 0.1.0
    CLI->>Engine: Runs headless ARM64 IL2CPP build
    Engine-->>CLI: Emits dist/my-quest-app-0.1.0.apk + build-metadata.json + sbom.cdx.json
    Dev->>CLI: quest verify dist/my-quest-app-0.1.0.apk --project-dir my-quest-app
    CLI-->>Dev: Audits QV-001..QV-006 -> Score 100/100 (READY)
    Dev->>CLI: quest ops deploy local-dev dist/my-quest-app-0.1.0.apk
    CLI->>Quest: Installs APK via adb install -r
```

### Commands
```pwsh
# 1. Scaffold repository (auto-generates AndroidManifest.xml and QuestForgeBootstrap.gen.*)
quest init my-quest-app --engine unity --package-id com.studio.myquestapp

# 2. After editing xr_features in quest_forge.yaml, synchronize generated files
quest sync --project-dir my-quest-app

# 3. Run sub-second static source, VRC manifest, and XR frame-budget checks
quest check --project-dir my-quest-app

# 4. Provision verified toolchain binaries
quest setup --project-dir my-quest-app --adb-version latest --adb-sha256 <sha256>

# 5. Run workstation diagnostics (pass --fix to auto-heal manifest/config drift)
quest doctor --fix --project-dir my-quest-app

# 6. Build APK, metadata, and CycloneDX SBOM locally
quest ci --project-dir my-quest-app --output-dir my-quest-app/dist --release-version 0.1.0

# 7. Run pre-flight VRC compliance verification
quest verify my-quest-app/dist/my-quest-app-0.1.0.apk --project-dir my-quest-app

# 8. Deploy to connected headset and inspect logs
quest ops discover --group local-dev
quest ops deploy local-dev my-quest-app/dist/my-quest-app-0.1.0.apk
quest ops logcat <serial> --out my-quest-app/dist/logcat.txt
```

### Declarative XR Codegen, Scene Authoring & Manifest Workflow (`quest sync` & `quest check`)

Like `machineweb` (`mweb`) and `swartch`, QuestForge eliminates boilerplate by generating engine manifests, `AndroidManifest.xml`, runtime XR startup code, and scene interaction controllers directly from `quest_forge.yaml` and `specs/*_xr.yaml`.

```mermaid
flowchart TD
    Manifest["quest_forge.yaml (xr_features, sdk, targets)"]
    SceneSpec["specs/main_arena_xr.yaml (Locomotion, Interactables, Spatial Panels)"]
    Sync["quest sync (Declarative XR Code Generator)"]
    AndroidManifest["Assets/Plugins/Android/AndroidManifest.xml (Auto-Generated VRC XML)"]
    Bootstrap["Assets/Generated/QuestForgeBootstrap.gen.cs (90/120Hz, Foveation, Passthrough)"]
    SceneGen["Assets/Generated/MainArenaXrScene.gen.cs (IMainArenaXrActions + MainArenaXrController)"]
    UserCode["Assets/Scripts/MainArenaActions.cs (Developer Implements Only IMainArenaXrActions)"]
    Check["quest check (QF-SPEC-001, QF-SYNC-001, QF-VRC-001, QF-PERF-001)"]

    Manifest --> Sync
    SceneSpec --> Sync
    Sync --> AndroidManifest
    Sync --> Bootstrap
    Sync --> SceneGen
    SceneGen --> UserCode
    AndroidManifest --> Check
    Bootstrap --> Check
    UserCode --> Check
```

#### 1. Declare Runtime Capabilities in `quest_forge.yaml`
```yaml
version: "1"
project:
  name: my-quest-app
  package_id: com.studio.myquestapp
  engine: unity
  engine_version: "2022.3.42f1"
targets:
  devices: [quest2, quest3, quest3s, questpro]
  min_sdk: 29
  target_sdk: 32
sdk:
  openxr: true
  meta_xr_sdk: "68.0.0"
xr_features:
  refresh_rate_hz: 90
  input_modes: [controllers, hands]
  passthrough: true
  spatial_anchors: true
  foveated_rendering: high
```

#### 2. Declare Scene Interactables & Spatial UI in `specs/main_arena_xr.yaml`
```yaml
scene: MainArena
locomotion:
  mode: teleport
  snap_turn_deg: 45
interactables:
  - id: energy_core
    kind: grabbable
    two_handed: true
    haptic_intensity: 80
    on_activate: OnEnergyCoreGrabbed
  - id: start_button
    kind: poke_button
    two_handed: false
    haptic_intensity: 50
    on_activate: OnStartButtonPoked
spatial_panels:
  - id: hud_panel
    title: "Mission HUD"
    distance_cm: 150
    follow_head: true
    actions:
      - id: reset_arena
        label: "Reset Arena"
        handler: OnResetArena
```

#### 3. Run `quest sync` (Auto-Generates All Boilerplate)
Running `quest sync` generates and locks:
- **`Assets/Plugins/Android/AndroidManifest.xml`** (or `Build/Android/AndroidManifest.xml` for Unreal) — VRC-compliant XML with `android.hardware.vr.headtracking`, `com.oculus.supportedDevices`, `com.oculus.intent.category.VR`, `HAND_TRACKING`, `PASSTHROUGH`, and `USE_ANCHOR_API`.
- **`Assets/Generated/QuestForgeBootstrap.gen.cs`** (or `Source/QuestForge/QuestForgeBootstrap.gen.h`) — Configures target frame rate, foveation, passthrough, and hand tracking automatically at startup.
- **`Assets/Generated/MainArenaXrScene.gen.cs`** (or `Source/QuestForge/MainArenaXrScene.gen.h`) — Generates the strongly-typed `IMainArenaXrActions` interface, `InteractableDescriptor[]`, `SpatialPanelDescriptor[]`, and `MainArenaXrController : MonoBehaviour` (with zero per-frame allocations).

#### 4. Implement Only Your Action Callbacks (`Assets/Scripts/MainArenaActions.cs`)
The developer never touches `AndroidManifest.xml` or low-level XR setup—only implementing the generated interface:
```csharp
using UnityEngine;
using QuestForge.Generated;

namespace QuestForge.App
{
    public sealed class MainArenaActions : MonoBehaviour, IMainArenaXrActions
    {
        public void OnEnergyCoreGrabbed()
        {
            Debug.Log("[MainArena] Energy core grabbed");
        }

        public void OnStartButtonPoked()
        {
            Debug.Log("[MainArena] Start button poked");
        }

        public void OnResetArena()
        {
            Debug.Log("[MainArena] Arena reset requested");
        }
    }
}
```

#### 5. Verify with `quest check` and `swartch`
Before building an APK, run `quest check` to verify specs, sync state, VRC rules, and VR frame budgets in milliseconds:
```pwsh
quest check --project-dir my-quest-app
swartch --vocabulary config/vocabulary.toml .
```

#### 6. Multi-Engine Codegen Matrix & Roadmap

`quest sync` and `quest check` work identically across **Unity**, **Unreal Engine**, and **Native Rust OpenXR** today, with additional targets tracked on the roadmap (`#18`):

| Engine Target | Status | `quest_forge.yaml` | Generated Bootstrap | Generated Scene Code (`specs/*_xr.yaml`) | User Handler File |
|---|---|---|---|---|---|
| **Unity (C# / IL2CPP)** | **Shipped** | `engine: unity` | `Assets/Generated/QuestForgeBootstrap.gen.cs` | `Assets/Generated/<Scene>XrScene.gen.cs` | `Assets/Scripts/MainArenaActions.cs` |
| **Unreal Engine (C++)** | **Shipped** | `engine: unreal` | `Source/QuestForge/QuestForgeBootstrap.gen.h` | `Source/QuestForge/<Scene>XrScene.gen.h` | `Source/QuestForge/MainArenaActions.h` |
| **Native Rust OpenXR** | **Shipped** | `engine: rust` | `src/questforge_bootstrap_gen.rs` | `src/<scene>_xr_scene_gen.rs` | `src/main_arena_actions.rs` |
| **Godot 4 OpenXR** | **Roadmap (`#18`)** | `engine: godot` | `addons/questforge/bootstrap.gen.gd` | `addons/questforge/<scene>_xr_scene.gen.gd` | `scripts/main_arena_actions.gd` |
| **Native C++ OpenXR** | **Roadmap (`#18`)** | `engine: cpp` | `include/questforge_bootstrap.gen.hpp` | `include/<scene>_xr_scene.gen.hpp` | `src/main_arena_actions.cpp` |
| **WebXR + `wgpu` (`mweb`)** | **Roadmap (`#18`)** | `engine: webxr` | `src/generated/questforge_bootstrap.gen.ts` | `src/generated/<scene>_xr_scene.gen.ts` | `src/features/main_arena_actions.ts` |

---

## 3. Actor 2: AI-Native XR Developer & Coding Agent

### Goal
Allow an AI coding assistant (Cursor, Claude Code, VS Code MCP client) to autonomously diagnose environment issues, scaffold projects, trigger builds, classify cryptic IL2CPP/NDK/Gradle errors, and verify APK compliance over JSON-RPC 2.0 (`quest mcp`).

### AI Agent Interaction Diagram

```mermaid
sequenceDiagram
    autonumber
    actor Dev as AI-Native Developer
    participant Agent as IDE AI Coding Agent
    participant MCP as quest mcp (JSON-RPC 2.0 stdio)
    participant Core as QuestForge Core Crates

    Dev->>Agent: "Why did my Quest 3 build fail and is the APK store-ready?"
    Agent->>MCP: tools/call {"name": "doctor_check", "arguments": {"strict": false}}
    MCP->>Core: DoctorRunner::run(QD-001..QD-009)
    Core-->>MCP: Structured JSON DoctorReport
    MCP-->>Agent: QD-001..QD-009 status + remediation steps
    Agent->>MCP: tools/call {"name": "analyze_build_failure", "arguments": {"log_text": "..."}}
    MCP->>Core: Classify NDK / IL2CPP / Gradle / Signing failure
    Core-->>MCP: Root cause + actionable fix
    MCP-->>Agent: Remediation instructions
    Agent->>MCP: tools/call {"name": "build_project", "arguments": {"release_version": "1.0.0"}}
    MCP->>Core: CiOrchestrator::execute_build
    Core-->>MCP: APK path + SHA-256 + CycloneDX SBOM
    Agent->>MCP: tools/call {"name": "verify_release", "arguments": {"apk_path": "dist/app-1.0.0.apk"}}
    MCP->>Core: ApkVerifier::verify (QV-001..QV-006)
    Core-->>MCP: Score 100/100 (READY)
    MCP-->>Agent: Verification report
    Agent-->>Dev: Explains fix applied and confirms 100/100 VRC compliance
```

### Exposed MCP Tools
| MCP Tool | Purpose |
|---|---|
| `doctor_check` | Executes `QD-001`–`QD-009` (supports `fix: true`) and returns structured JSON diagnostics |
| `sync_project` | Synchronizes `quest_forge.yaml` (`xr_features`, `sdk`, `targets`) into `AndroidManifest.xml`, `QuestForgeBootstrap.gen.*`, and engine manifests |
| `check_source` | Runs sub-second static source gates (`QF-SYNC-001`, `QF-VRC-001`, `QF-PERF-001`) |
| `init_project` | Scaffolds a Unity or Unreal Quest project with `quest_forge.yaml` |
| `build_project` | Executes `CiOrchestrator` to produce an APK, metadata, and SBOM |
| `verify_release` | Runs `QV-001`–`QV-006` static verification on an `.apk` |
| `generate_pipeline` | Emits GitHub Actions, Azure DevOps, or Jenkins pipeline definitions |
| `analyze_build_failure` | Classifies Unity, Unreal, Gradle, NDK, IL2CPP, and signing errors |
| `search_docs` | Searches QuestForge VRC, doctor, and configuration documentation |

---

## 4. Actor 3: Platform & DevOps Engineer

### Goal
Standardize CI/CD pipelines across GitHub Actions, Azure DevOps, or Jenkins without vendor lock-in; enforce zero-trust SHA-256 binary provisioning; enforce `swartch` Architecture 4.0 rules; generate CycloneDX 1.5 SBOMs and SARIF reports; and automate Meta Horizon Store channel promotion.

### CI/CD Pipeline Flowchart

```mermaid
flowchart TD
    PR["Git Push / Pull Request"] --> Runner["CI Runner (scripts/ci.sh)"]
    Runner --> Swartch["1. swartch Architecture 4.0 Gate"]
    Swartch --> Setup["2. quest setup (Verify SHA-256 Toolchains)"]
    Setup --> DoctorCI["3. quest doctor --ci --strict --json"]
    DoctorCI --> Build["4. quest ci (Headless Build + SBOM + Metadata)"]
    Build --> VerifySARIF["5. quest verify --format sarif & json"]
    VerifySARIF --> Gate{"Score >= Policy & 0 Critical?"}
    Gate -->|"No"| Fail["Fail Pipeline & Upload SARIF Diagnostics"]
    Gate -->|"Yes"| Publish["6. POST /api/builds & /api/verify to quest serve"]
    Publish --> Store["7. ovr-platform-util Upload (ALPHA -> BETA -> RC -> PROD)"]
```

### Commands
```pwsh
# Generate CI pipeline configuration for your provider
quest ci --project-dir my-quest-app --emit-pipeline github
quest ci --project-dir my-quest-app --emit-pipeline azure
quest ci --project-dir my-quest-app --emit-pipeline jenkins

# Execute strict CI pipeline steps (identical across all CI providers)
quest setup --project-dir my-quest-app --adb-version 35.0.2 --adb-sha256 <sha256>
quest doctor --ci --strict --project-dir my-quest-app --json
quest ci --project-dir my-quest-app --output-dir dist --release-version 1.2.0 --upload-channel ALPHA --app-id $META_APP_ID
quest verify dist/my-quest-app-1.2.0.apk --project-dir my-quest-app --format sarif --out dist/verify.sarif.json
```

---

## 5. Actor 4: XR QA & Release Manager

### Goal
Audit release readiness scorecards (HTML/PDF/CLI), discover lab headsets and Android emulators, deploy candidate APKs to device groups (`qa-lab`, `staging-floor`), capture device telemetry/logcat/screenshots, and execute one-command rollbacks if regressions appear.

### QA Fleet & Release State Diagram

```mermaid
stateDiagram-v2
    [*] --> CandidateBuilt: quest ci produces APK
    CandidateBuilt --> Verified: quest verify (Score = 100)
    CandidateBuilt --> Rejected: quest verify (QV-001..QV-006 Fail)
    Verified --> QueuedForLab: POST /api/deployments or quest ops deploy
    QueuedForLab --> DeployingToFleet: quest ops / agent installs APK
    DeployingToFleet --> HealthyInLab: Telemetry OK + Smoke Test Pass
    DeployingToFleet --> RegressionDetected: Crash in logcat / Thermal Alert
    RegressionDetected --> RolledBack: quest ops rollback <group>
    HealthyInLab --> PromotedToStore: Release Manager approves RC / PROD
    RolledBack --> [*]
    Rejected --> [*]
    PromotedToStore --> [*]
```

### QA Workflow Diagram

```mermaid
flowchart LR
    Scorecard["1. Review HTML / PDF Scorecard"] --> Discover["2. quest ops discover --group qa-lab"]
    Discover --> Deploy["3. quest ops deploy qa-lab app.apk"]
    Deploy --> Inspect["4. quest ops telemetry / logcat / screenshot"]
    Inspect --> Decision{"Pass QA Smoke Test?"}
    Decision -->|"Pass"| Approve["5. Approve Store Channel Promotion"]
    Decision -->|"Fail"| Rollback["5. quest ops rollback qa-lab"]
```

### Commands
```pwsh
# 1. Generate human-readable HTML and PDF VRC compliance scorecards
quest verify dist/my-quest-app-1.2.0.apk --project-dir my-quest-app --format html --out dist/scorecard.html
quest verify dist/my-quest-app-1.2.0.apk --project-dir my-quest-app --format pdf --out dist/scorecard.pdf

# 2. Discover connected Quest headsets and Android emulators
quest ops discover --group qa-lab

# 3. Deploy candidate APK to the QA lab group
quest ops deploy qa-lab dist/my-quest-app-1.2.0.apk

# 4. Collect headset health telemetry, logcat logs, and screenshots
quest ops telemetry <serial>
quest ops logcat <serial> --lines 500 --out dist/qa-logcat.log
quest ops screenshot <serial> --out dist/qa-screenshot.png

# 5. Roll back the entire device group to the previous known-good APK if needed
quest ops rollback qa-lab
```

---

## 6. Actor 5: Enterprise Security, Compliance & Lab Operations Lead

### Goal
Operate the centralized **QuestForge Control Plane** (`quest serve` + `web/` dashboard) backed by PostgreSQL, enforce JWT/OIDC authentication and role-based access control (`Admin`, `Operator`, `Viewer`), run automated lab agents (`quest ops agent`), and maintain an immutable audit trail (`/api/audit`) for regulatory compliance.

### Enterprise Governance & Lab Agent Sequence Diagram

```mermaid
sequenceDiagram
    autonumber
    actor SecLead as Enterprise Security / Ops Lead
    participant Web as web/ Dashboard (machineweb)
    participant Server as quest serve (Control Plane)
    participant PG as PostgreSQL (questforge DB)
    participant LabAgent as quest ops agent (Lab Host)
    participant Fleet as Quest Lab Headsets (ADB)

    SecLead->>Server: Start quest serve (--database-url postgres://... --jwt-secret ...)
    Server->>PG: Initialize schema (projects, builds, verify_reports, deployments, audit_log)
    SecLead->>Web: Open http://127.0.0.1:8090/
    Web->>Server: GET /api/projects, /api/builds, /api/reports, /api/deployments, /api/audit
    Server->>PG: Query persisted state
    PG-->>Server: Rows
    Server-->>Web: JSON payloads rendered in dashboard
    SecLead->>Web: Queue deployment (Group: qa-lab, APK: v1.2.0, Role: Operator/Admin JWT)
    Web->>Server: POST /api/deployments (Bearer JWT)
    Server->>PG: Persist deployment (status = queued) + append immutable audit_log entry
    loop Poll Interval
        LabAgent->>Server: GET /api/deployments
        Server-->>LabAgent: Queued deployment for group "qa-lab"
        LabAgent->>Fleet: adb devices -> adb install -r app-1.2.0.apk
        Fleet-->>LabAgent: Install success + battery/storage/thermal telemetry
    end
    SecLead->>Server: GET /api/audit & GET /metrics
    Server-->>SecLead: Immutable actor/action/timestamp audit trail + Prometheus metrics
```

### RBAC Permission Matrix

```mermaid
flowchart LR
    subgraph Roles["JWT / RBAC Roles"]
        Viewer["Viewer Role"]
        Operator["Operator Role"]
        Admin["Admin Role"]
    end

    subgraph Endpoints["Control Plane Capabilities"]
        Read["GET /health, /metrics, /api/projects, /api/builds, /api/reports, /api/deployments, /api/audit"]
        Write["POST /api/projects, /api/builds, /api/reports, /api/verify, /api/deployments"]
    end

    Viewer --> Read
    Operator --> Read
    Operator --> Write
    Admin --> Read
    Admin --> Write
```

### Commands
```pwsh
# 1. Start the Control Plane with PostgreSQL persistence, JWT RBAC, and web dashboard
quest serve `
  --addr 127.0.0.1:8090 `
  --web-root web/dist `
  --database-url postgres://questforge:questforge@127.0.0.1:5432/questforge `
  --jwt-secret $env:QUESTFORGE_JWT_SECRET

# 2. Start the lab agent on the QA lab host to auto-sync queued deployments
quest ops agent `
  --server http://127.0.0.1:8090 `
  --group qa-lab `
  --interval-secs 15
```
