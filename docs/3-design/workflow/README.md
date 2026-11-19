# QuestForge Role Workflows

Visual design workflows for the five role-based actors in the **QuestForge** ecosystem across **Meta Quest 2, Quest Pro, Quest 3, and Quest 3S (Meta Horizon OS)**.

---

## Multi-Role Ecosystem Overview

```mermaid
flowchart TD
    subgraph Roles["Role-Based Actors (--role)"]
        Studio["1. Studio (XR Game Teams)"]
        Developer["2. Developer (AI-Native XR Creators)"]
        Platform["3. Platform (Platform & DevOps)"]
        QA["4. QA & Release (QA & Release Managers)"]
        Enterprise["5. Enterprise (Security & Ops Leads)"]
    end

    subgraph CLI["QuestForge CLI & MCP (quest)"]
        Init["quest init (unity | unreal | rust)"]
        Sync["quest sync (XR + OpenXR Codegen)"]
        Check["quest check (SPEC/SYNC/VRC/OXR/PERF)"]
        Setup["quest setup (Zero-Trust SHA-256)"]
        Doctor["quest doctor --fix (QD-001..QD-009)"]
        MCP["quest mcp & quest agent --role (14 JSON-RPC 2.0 Tools)"]
        CI["quest ci (Build + CycloneDX SBOM)"]
        Verify["quest verify (QV-001..QV-006)"]
        Ops["quest ops (ADB Fleet + store + hms-push)"]
    end

    subgraph ControlPlane["QuestForge Control Plane"]
        Serve["quest serve (REST API + JWT RBAC + PostgreSQL)"]
        Web["web/ Dashboard (machineweb UI)"]
        Agent["quest ops agent (Lab Polling Daemon)"]
    end

    subgraph Targets["Execution & Distribution Targets"]
        Headsets["Meta Quest Fleet & OpenXR Simulator"]
        Store["Meta Horizon Store (ALPHA / BETA / RC / PROD)"]
        HMS["Meta HMS / ManageXR / ArborXR (Enterprise MDM)"]
    end

    Studio --> Init & Sync & Check & CI & Verify
    Developer --> MCP
    MCP --> Init & Sync & Check & Setup & Doctor & CI & Verify & Ops
    Platform --> Setup & Doctor & CI
    CI --> Store
    QA --> Verify & Ops & Web
    Ops --> Headsets & Store & HMS
    Enterprise --> Serve & Web & Ops
    Serve --> Agent --> Headsets
```

---

## Standalone Role Workflows

| Role (`--role`) | Target Persona | Workflow Document |
|---|---|---|
| **`studio`** | XR Game Teams (Unity, Unreal, Native Rust OpenXR) | [Studio Workflow](studio.md) |
| **`developer`** | AI-Native XR Creators & IDE Coding Agents | [Developer Workflow](developer.md) |
| **`platform`** | Platform & DevOps Engineers | [Platform Workflow](platform.md) |
| **`qa`** | XR QA & Release Managers | [QA & Release Workflow](qa.md) |
| **`enterprise`** | Enterprise Security, Compliance & Lab Operations Leads | [Enterprise Workflow](enterprise.md) |
