# QuestForge

[![CI](https://github.com/sweengineeringlabs/questforge/actions/workflows/ci.yml/badge.svg)](https://github.com/sweengineeringlabs/questforge/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/sweengineeringlabs/questforge)](https://github.com/sweengineeringlabs/questforge/releases/latest)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**Autonomous, role-based AI agents for Meta Quest & Meta Horizon OS** — guiding VR teams from setup and OpenXR verification to `100/100` release readiness and headset rollout.

![QuestForge Hero](docs/assets/devpost-hero.jpg)

---

## Why QuestForge?

Building for **Meta Quest** shouldn't mean wrestling with OpenXR permissions, brittle SDK setups, or surprise Meta Store rejections. QuestForge puts **role-based AI agents** (`Gemini`, `OpenAI`, `Anthropic`) directly in charge of your VR lifecycle—so creators can focus on worlds, presence, and gameplay.

| AI Agent Role | Built For | Focus |
|---|---|---|
| **Studio** | XR Game Teams | Project setup, OpenXR capability sync, build checks & store readiness |
| **QA & Release** | QA & Release Managers | `0–100` Meta Horizon VRC readiness scorecards & lab headset rollout |
| **Platform** | Platform & DevOps | Environment parity, CI workflows & automated build diagnostics |
| **Enterprise** | Training & Simulation | Supply-chain auditability, SBOMs & controlled fleet distribution |
| **Developer** | AI-Native XR Creators | Interactive copilot across all QuestForge VR tools |

---

## Quick Start

### Windows (PowerShell)

```pwsh
# 1. Download and extract quest.exe from GitHub Releases
Invoke-WebRequest -Uri "https://github.com/sweengineeringlabs/questforge/releases/latest/download/quest-x86_64-pc-windows-msvc.zip" -OutFile quest.zip
Expand-Archive -Path quest.zip -DestinationPath . -Force

# 2. Authenticate and run a role-based XR agent
.\quest.exe auth --provider gemini
.\quest.exe agent --role studio --provider gemini --prompt "Inspect my Quest project and verify Meta Quest readiness"
```

### Linux & WSL (Bash)

```bash
# 1. Download and extract quest from GitHub Releases
curl -fsSL https://github.com/sweengineeringlabs/questforge/releases/latest/download/quest-x86_64-unknown-linux-gnu.tar.gz | tar -xz
chmod +x ./quest

# 2. Authenticate and run a role-based XR agent
./quest auth --provider gemini
./quest agent --role studio --provider gemini --prompt "Inspect my Quest project and verify Meta Quest readiness"
```

See the **[Runbook & Playbook](docs/6-operations/runbook.md)** and **[Developer Guide](docs/4-development/developer_guide.md)** for full Windows and Linux / WSL workflows.

---

## Overview & Documentation

Explore the **[QuestForge Overview](docs/overview.md)** — the central index to our product concept, architecture, workflows, developer guide, and runbooks.
