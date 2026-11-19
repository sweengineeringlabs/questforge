# Platform & DevOps Workflow (`--role platform`)

**Audience**: Platform engineers, CI/CD maintainers, and DevOps leads

**Target Persona:** Platform & DevOps Engineers standardizing Quest CI/CD across GitHub Actions, Azure DevOps, or Jenkins with zero-trust SHA-256 binary provisioning, CycloneDX 1.5 SBOMs, and SARIF compliance gates.

---

## Table of Contents

- [1. Provider-Agnostic CI/CD Pipeline Flow](#1-provider-agnostic-cicd-pipeline-flow)
- [2. Build Failure Diagnostics & Remediation Loop](#2-build-failure-diagnostics--remediation-loop)

---

## 1. Provider-Agnostic CI/CD Pipeline Flow

```mermaid
flowchart TD
    PR["Git Push / Pull Request"] --> Runner["CI Runner (scripts/ci.sh)"]
    Runner --> Setup["1. quest setup (Verify SHA-256 Toolchains: adb, cmdline-tools, ovr-platform-util)"]
    Setup --> DoctorCI["2. quest doctor --ci --strict --json"]
    DoctorCI --> Check["3. quest check (QF-SPEC / SYNC / VRC / OXR / PERF)"]
    Check --> Build["4. quest ci (Headless ARM64 Build + CycloneDX SBOM + Metadata)"]
    Build --> VerifySARIF["5. quest verify --format sarif & json"]
    VerifySARIF --> Gate{"Score == 100 & 0 Critical?"}
    Gate -->|"No"| Fail["Fail Pipeline & Upload SARIF Diagnostics"]
    Gate -->|"Yes"| Publish["6. POST /api/builds & /api/verify to quest serve"]
    Publish --> Store["7. quest ops store / ovr_store (ALPHA -> BETA -> RC -> PROD)"]
```

---

## 2. Build Failure Diagnostics & Remediation Loop

```mermaid
sequenceDiagram
    autonumber
    actor DevOps as Platform Engineer
    participant Agent as quest agent --role platform
    participant MCP as quest mcp
    participant Toolchain as questforge-toolchain / doctor / ci

    DevOps->>Agent: "Verify CI runner parity and diagnose the failed Android NDK build"
    Agent->>MCP: tools/call {"name": "setup_toolchain", "arguments": {"adb_version": "latest", "ovr_version": "latest"}}
    MCP->>Toolchain: Provision & verify SHA-256 binaries (Windows or Linux / WSL)
    Toolchain-->>MCP: Cached binary paths
    Agent->>MCP: tools/call {"name": "doctor_check", "arguments": {"strict": true}}
    MCP->>Toolchain: Check QD-001..QD-009
    Toolchain-->>MCP: Doctor report
    Agent->>MCP: tools/call {"name": "analyze_build_failure", "arguments": {"log_text": "..."}}
    MCP->>Toolchain: Classify NDK / IL2CPP / Gradle / Signing error
    Toolchain-->>MCP: Root cause + remediation steps
    Agent->>MCP: tools/call {"name": "generate_pipeline", "arguments": {"provider": "github"}}
    MCP-->>Agent: Pipeline YAML
    Agent-->>DevOps: Remediation report + updated CI pipeline
```

See the **[Operations Runbook](../../6-operations/runbook.md)** for full CI and workstation commands.
