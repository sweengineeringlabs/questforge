# Enterprise Security & Operations Workflow (`--role enterprise`)

**Target Persona:** Enterprise Security, Compliance, and Lab Operations Leads running the **QuestForge Control Plane** (`quest serve` + `web/` dashboard) backed by PostgreSQL, enforcing JWT RBAC and immutable audit logs, and rolling out verified builds to enterprise headset fleets via **Meta HMS (`graph.oculus.com`)**, **ManageXR (`managexrapi.com`)**, or **ArborXR (`api.xrdm.app`)**.

---

## 1. Enterprise Control Plane, Lab Agent & MDM Sequence

```mermaid
sequenceDiagram
    autonumber
    actor SecLead as Enterprise Security / Ops Lead
    participant Web as web/ Dashboard (machineweb)
    participant Server as quest serve (Control Plane)
    participant PG as PostgreSQL (questforge DB)
    participant LabAgent as quest ops agent (Lab Host)
    participant Fleet as Quest Lab Headsets (ADB)
    participant MDM as Meta HMS / ManageXR / ArborXR

    SecLead->>Server: Start quest serve (--database-url postgres://... --jwt-secret ...)
    Server->>PG: Initialize schema (projects, builds, verify_reports, deployments, audit_log)
    SecLead->>Web: Open Control Plane Dashboard
    Web->>Server: GET /api/projects, /api/builds, /api/reports, /api/deployments, /api/audit
    Server->>PG: Query persisted state
    PG-->>Server: Rows
    Server-->>Web: JSON payloads rendered in dashboard
    SecLead->>Web: Queue lab deployment (Group: qa-lab, APK: v1.2.0, Bearer JWT)
    Web->>Server: POST /api/deployments
    Server->>PG: Persist deployment (status = queued) + append immutable audit_log entry
    loop Poll Interval
        LabAgent->>Server: GET /api/deployments
        Server-->>LabAgent: Queued deployment for group "qa-lab"
        LabAgent->>Fleet: adb devices -> adb install -r app-1.2.0.apk
        Fleet-->>LabAgent: Install success + battery/storage/thermal telemetry
    end
    SecLead->>Server: Promote verified build to Enterprise MDM (quest ops hms-push / publish_mdm)
    Server->>MDM: HTTPS Bearer push (APK SHA-256 + release channel + target device group)
    MDM-->>Server: OTA rollout confirmation
    SecLead->>Server: GET /api/audit & GET /metrics
    Server-->>SecLead: Immutable actor/action/timestamp audit trail + Prometheus metrics
```

---

## 2. RBAC Permission Matrix

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
    Operator --> Read & Write
    Admin --> Read & Write
```

See the **[Operations Runbook](../../6-operations/runbook.md)** for full Control Plane, PostgreSQL, JWT, and Enterprise MDM commands.
