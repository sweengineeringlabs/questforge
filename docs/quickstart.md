# QuestForge Quick Start

**Audience**: All QuestForge users, XR developers, platform engineers, and QA leads

Get up and running with **QuestForge** on **Windows** or **Linux / WSL**.

---

## Table of Contents

- [1. Install `quest`](#1-install-quest)
  - [Windows (PowerShell)](#windows-powershell)
  - [Linux & WSL (Bash)](#linux--wsl-bash)
- [2. Authenticate & Launch Your AI Agent](#2-authenticate--launch-your-ai-agent)
- [3. Next Steps](#3-next-steps)

---

## 1. Install `quest`

### Windows (PowerShell)

```pwsh
Invoke-WebRequest -Uri "https://github.com/sweengineeringlabs/questforge/releases/latest/download/quest-x86_64-pc-windows-msvc.zip" -OutFile quest.zip
Expand-Archive -Path quest.zip -DestinationPath . -Force
.\quest.exe --version
```

### Linux & WSL (Bash)

```bash
curl -fsSL https://github.com/sweengineeringlabs/questforge/releases/latest/download/quest-x86_64-unknown-linux-gnu.tar.gz | tar -xz
chmod +x ./quest
./quest --version
```

---

## 2. Authenticate & Launch Your AI Agent

### Windows (PowerShell)

```pwsh
# Authenticate with your AI provider (gemini, openai, or anthropic)
.\quest.exe auth --provider gemini

# Scaffold a sample Meta Quest OpenXR project (or point --project-dir at an existing project)
.\quest.exe init my-quest-app --engine rust --package-id com.studio.myquestapp --skip-doctor

# Launch the Studio AI agent to inspect and verify readiness
.\quest.exe agent --role studio --provider gemini --project-dir my-quest-app --prompt "Inspect my Quest project and verify Meta Quest readiness"
```

### Linux & WSL (Bash)

```bash
# Authenticate with your AI provider (gemini, openai, or anthropic)
./quest auth --provider gemini

# Scaffold a sample Meta Quest OpenXR project (or point --project-dir at an existing project)
./quest init my-quest-app --engine rust --package-id com.studio.myquestapp --skip-doctor

# Launch the Studio AI agent to inspect and verify readiness
./quest agent --role studio --provider gemini --project-dir my-quest-app --prompt "Inspect my Quest project and verify Meta Quest readiness"
```

---

## 3. Next Steps

- **[Developer Guide](4-development/developer_guide.md)** — Scaffold (`quest init`), sync (`quest sync`), and verify (`quest check`, `quest verify`) XR projects
- **[Operations Runbook](6-operations/runbook.md)** — Control plane (`quest serve`), lab headset fleet (`quest ops`), Meta Horizon Store releases (`quest ops store-upload`), and enterprise MDM push (`quest ops hms-push`)
- **[Documentation Hub](README.md)** — Architecture, role workflows, specifications, and glossary
