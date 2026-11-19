# QA & Release Workflow (`--role qa`)

**Audience**: XR QA engineers, release managers, and lab fleet operators

**Target Persona:** XR QA & Release Managers auditing `0–100` Meta Horizon VRC scorecards, autonomously remediating manifest/sync drift (`sync_project` → `build_project` → `verify_release`), deploying candidate APKs to lab headset fleets, and promoting verified builds to Meta Horizon Store release channels.

---

## Table of Contents

- [1. QA Release Readiness State Machine](#1-qa-release-readiness-state-machine)
- [2. QA Fleet Validation & Horizon Store Promotion Flow](#2-qa-fleet-validation--horizon-store-promotion-flow)
- [3. Autonomous QA Agent Remediation, Promotion & Store Flow](#3-autonomous-qa-agent-remediation-promotion--store-flow)
- [4. Release Channel Promotion & Tester Onboarding](#4-release-channel-promotion--tester-onboarding)
- [5. Multi-Tiered Input & Gesture Testing Surface](#5-multi-tiered-input--gesture-testing-surface)

---

## 1. QA Release Readiness State Machine

```mermaid
stateDiagram-v2
    [*] --> CandidateBuilt: quest ci produces APK
    CandidateBuilt --> Verified: quest verify (Score = 100/100)
    CandidateBuilt --> AutoRemediating: quest verify fails (manifest/sync drift)
    AutoRemediating --> CandidateBuilt: quest sync + quest check + quest ci rebuild
    Verified --> QueuedForLab: POST /api/deployments or quest ops deploy
    QueuedForLab --> DeployingToFleet: quest ops / agent installs APK via ADB
    DeployingToFleet --> HealthyInLab: Telemetry OK + Smoke Test Pass
    DeployingToFleet --> RegressionDetected: Crash in logcat / Thermal Alert
    RegressionDetected --> RolledBack: quest ops rollback <group>
    HealthyInLab --> PromotedToStore: quest ops store (ALPHA -> BETA -> RC -> PRODUCTION)
    RolledBack --> [*]
    PromotedToStore --> [*]
```

---

## 2. QA Fleet Validation & Horizon Store Promotion Flow

```mermaid
flowchart LR
    Scorecard["1. quest verify (100/100 HTML / PDF / JSON Scorecard)"] --> Discover["2. quest ops discover --group qa-lab"]
    Discover --> Deploy["3. quest ops deploy qa-lab app.apk"]
    Deploy --> Inspect["4. quest ops telemetry / logcat / screenshot"]
    Inspect --> Decision{"Pass QA Smoke Test?"}
    Decision -->|"Pass"| Store["5. quest ops store --action upload-quest-build"]
    Decision -->|"Fail"| Rollback["5. quest ops rollback qa-lab"]
```

---

## 3. Autonomous QA Agent Remediation, Promotion & Store Flow

```mermaid
sequenceDiagram
    autonumber
    actor QA as QA & Release Manager
    participant Agent as quest agent --role qa
    participant MCP as quest mcp
    participant Core as Verify / Init / CI / Ops Crates
    participant Store as Meta Horizon Store (ovr-platform-util)

    QA->>Agent: "Verify dist/app.apk; if below 100/100 fix and rebuild, then upload to ALPHA"
    Agent->>MCP: tools/call {"name": "verify_release", "arguments": {"apk_path": "dist/app.apk"}}
    MCP->>Core: ApkVerifier::verify (QV-001..QV-010)
    Core-->>MCP: Score < 100 (or 100/100)
    opt If manifest or sync drift detected
        Agent->>MCP: tools/call {"name": "sync_project", "arguments": {}}
        Agent->>MCP: tools/call {"name": "check_source", "arguments": {}}
        Agent->>MCP: tools/call {"name": "build_project", "arguments": {"release_version": "1.0.1"}}
        Agent->>MCP: tools/call {"name": "verify_release", "arguments": {"apk_path": "dist/app-1.0.1.apk"}}
        Core-->>MCP: Score 100/100 (READY)
    end
    Agent->>MCP: tools/call {"name": "upload_store", "arguments": {"channel": "ALPHA"}}
    MCP->>Core: Provision ovr-platform-util (if missing) & translate WSL paths if on WSL
    Core->>Store: ovr-platform-util upload-quest-build --apk ... --channel ALPHA
    Store-->>Core: Upload complete (Build ID: 1440416305830062)
    Core-->>MCP: Structured StoreUploadReceipt (build_id, test_results_url, invite_url)
    MCP-->>Agent: Store upload report + persistent deployments.json
    Agent-->>QA: Confirmed 100/100 VRC compliance, build ID & test results URL
```

---

## 4. Release Channel Promotion & Tester Onboarding

QA teams can promote validated builds across channels and onboard internal/external testers:

```powershell
# Promote active ALPHA build to BETA autonomously
quest agent --role qa --goal promote --channel BETA

# Invite testers via Meta Horizon Graph API
quest ops testers --channel ALPHA --action invite --email "qa-tester@example.com"

# Generate direct developer portal onboarding link
quest ops testers --channel ALPHA --action deep-link

# Connect to cable-free lab headsets over Wi-Fi
quest ops connect 192.168.1.50:5555
quest ops pair 192.168.1.50:37123 --code 123456
```

---

## 5. Multi-Tiered Input & Gesture Testing Surface

Following [ADR-002](../adr/002_stratified_input_and_gesture_kinematics.md), QA teams and evaluators are never blocked by physical headset availability:

| Testing Tier | Device Surface | Gesture Verification Method |
|---|---|---|
| **Tier 1: Physical Headset** | Meta Quest 2 / 3 / 3S / Pro | Optical hand tracking via OpenXR `XR_EXT_hand_tracking` joint velocity & oscillation. |
| **Tier 2: Physical Android Device** | Standard Android Phone / Tablet | Hands-free wave over front IR proximity sensor (`Sensor.TYPE_PROXIMITY`, $< 5\text{ cm}$). |
| **Tier 3: Headless CI / AVD** | `mobile-emulator` Android AVD | Touch fling swipe listener (`GestureDetector`) or synthetic JNI entry point (`nativeOnGesture(1)`). |

See the **[Operations Runbook](../../6-operations/runbook.md)** for full QA fleet and Horizon Store commands.
