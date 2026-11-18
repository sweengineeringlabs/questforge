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
        Setup["quest setup (Zero-Trust SHA-256)"]
        Doctor["quest doctor (QD-001..QD-009)"]
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
    Dev --> Setup
    Dev --> Doctor
    Dev --> CI
    Dev --> Verify

    AIDev --> MCP
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
    A["1. quest init"] --> B["2. quest setup"]
    B --> C["3. quest doctor"]
    C --> D["4. Edit Unity / Unreal Scene"]
    D --> E["5. quest ci (Local APK Build)"]
    E --> F["6. quest verify (QV-001..QV-006)"]
    F -->|"Score = 100 / Ready"| G["7. quest ops deploy (USB / Wi-Fi Headset)"]
    F -->|"Failed Gate"| D
    G --> H["8. Open Pull Request"]
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
# 1. Scaffold repository
quest init my-quest-app --engine unity --package-id com.studio.myquestapp

# 2. Provision verified toolchain binaries
quest setup --project-dir my-quest-app --adb-version latest --adb-sha256 <sha256>

# 3. Run workstation diagnostics
quest doctor --project-dir my-quest-app

# 4. Build APK, metadata, and CycloneDX SBOM locally
quest ci --project-dir my-quest-app --output-dir my-quest-app/dist --release-version 0.1.0

# 5. Run pre-flight VRC compliance verification
quest verify my-quest-app/dist/my-quest-app-0.1.0.apk --project-dir my-quest-app

# 6. Deploy to connected headset and inspect logs
quest ops discover --group local-dev
quest ops deploy local-dev my-quest-app/dist/my-quest-app-0.1.0.apk
quest ops logcat <serial> --out my-quest-app/dist/logcat.txt
```

### XR Coding, Scene Authoring & Manifest Workflow

During step 4 of the inner loop, developers write gameplay, interaction, and rendering code while keeping three project artifacts synchronized with `quest doctor` (`QD-001`, `QD-006`, `QD-009`) and `quest verify` (`QV-001`–`QV-006`).

```mermaid
flowchart TD
    Manifest["quest_forge.yaml (Single Source of Truth)"] --> Unity["Unity C# (Assets/Scripts/ & Packages/manifest.json)"]
    Manifest --> Unreal["Unreal C++ / Blueprints (Source/ & QuestApp.uproject)"]
    Unity --> AndroidManifest["Assets/Plugins/Android/AndroidManifest.xml"]
    Unreal --> AndroidManifest
    Unity --> Builder["Assets/Editor/QuestForgeBuild.cs (IL2CPP + ARM64)"]
    Unreal --> UAT["RunUAT BuildCookRun (ASTC + ARM64)"]
    AndroidManifest --> VRC["Meta VRC Compliance (QV-003, QV-004, QV-005)"]
    Builder --> APK["Signed Release APK"]
    UAT --> APK
    VRC --> APK
```

#### 1. Project Manifest (`quest_forge.yaml`)
Every engine version, target Quest headset, SDK version, and signing variable is declared in `quest_forge.yaml` so `quest doctor` (`QD-001`, `QD-006`) can catch configuration drift immediately:

```yaml
version: "1"
project:
  name: my-quest-app
  package_id: com.studio.myquestapp
  engine: unity
  engine_version: "2022.3.42f1"
targets:
  devices: [quest2, quest3, quest3s, questpro]
  min_android_api: 32
  target_android_api: 34
sdk:
  openxr: true
  meta_xr_sdk: "68.0.0"
```

#### 2. Writing Unity XR C# Code (`Assets/Scripts/` & `Assets/Editor/QuestForgeBuild.cs`)
- **Runtime XR Scripts (`Assets/Scripts/QuestRigController.cs`):** Write OpenXR / Meta XR SDK (`com.meta.xr.sdk.all`, `com.unity.xr.openxr`) scripts targeting 72/90/120 Hz display refresh rates, controller/hand-tracking input, and passthrough layers:
  ```csharp
  using UnityEngine;
  using UnityEngine.XR;

  namespace MyQuestApp
  {
      public sealed class QuestRigController : MonoBehaviour
      {
          [SerializeField] private float targetRefreshRate = 90f;

          private void Start()
          {
              Application.targetFrameRate = Mathf.RoundToInt(targetRefreshRate);
              Debug.Log($"[QuestForge] XR Device Active: {XRSettings.loadedDeviceName}");
          }
      }
  }
  ```
- **Headless Build Hook (`Assets/Editor/QuestForgeBuild.cs`):** Scaffolded by `quest init` and invoked automatically by `quest ci` (`-batchmode -nographics -executeMethod QuestForge.Build.BuildQuestApk`). It enforces `ScriptingImplementation.IL2CPP` and `AndroidArchitecture.ARM64` so the output binary always satisfies `QV-003` (`arm64-v8a` ABI compliance).

#### 3. Writing Unreal Engine C++ / Blueprints (`QuestApp.uproject`)
- Enable the `OpenXR` and `MetaXR` plugins in `QuestApp.uproject` and configure `DefaultEngine.ini` for Vulkan / Mobile Multi-View and ASTC texture compression (`Android_ASTC`).
- `quest ci` invokes `RunUAT BuildCookRun -platform=Android -cookflavor=ASTC -clientconfig=Shipping` to compile C++ modules and cook `.pak` assets into a signed APK.

#### 4. Authoring a VRC-Compliant `AndroidManifest.xml`
To pass `QV-004` (Meta VRC Manifest Policy) and `QV-005` (Permissions Audit) on the first try:
- Set `package="com.studio.myquestapp"` matching `project.package_id` in `quest_forge.yaml`.
- Ensure `android:debuggable="false"` on `<application>` for release builds.
- Declare `<uses-feature android:name="android.hardware.vr.headtracking" android:required="true" android:version="1" />`.
- Include the Horizon OS VR launch category `<category android:name="com.oculus.intent.category.VR" />`.
- Avoid unapproved sensitive Android permissions (`READ_CONTACTS`, `READ_SMS`, `ACCESS_FINE_LOCATION`, `SYSTEM_ALERT_WINDOW`).

#### 5. Enforcing Repository Architecture While Coding (`swartch`)
Before committing code changes, run the repository architecture check (`QD-009`) against `config/vocabulary.toml`:
```pwsh
swartch --vocabulary config/vocabulary.toml .
```

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
| `doctor_check` | Executes `QD-001`–`QD-009` and returns structured JSON diagnostics |
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
