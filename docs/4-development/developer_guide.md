# QuestForge Developer Guide

Guide for both **Quest / Horizon OS application developers** using the `quest` CLI and **platform contributors** developing QuestForge itself.

---

## Part 1: For Quest / Horizon OS App Developers

### 1. Initialize a New Quest Project
```pwsh
quest init my-quest-app --engine unity --package-id com.acme.myquestapp
```
What `quest init` generates for Unity:
- `quest_forge.yaml` — single source of truth for engine, SDK/NDK/JDK, target Quest devices, and signing env vars
- `ProjectSettings/ProjectVersion.txt` & `Packages/manifest.json` (`com.meta.xr.sdk.all`, `com.unity.xr.openxr`)
- `Assets/Editor/QuestForgeBuild.cs` — headless Unity IL2CPP ARM64 builder (`QuestForge.Build.BuildQuestApk`) invoked automatically by `quest ci`
- `.gitignore` & `.gitattributes` (Git LFS rules for `.fbx`, `.png`, `.wav`, `.apk`, `.asset`)
- `scripts/ci.sh` — vendor-agnostic CI script

### 2. Provision Toolchain & Check Your Workstation
```pwsh
# Install adb into ~/.questforge/cache (pass --adb-sha256 for zero-trust verification)
quest setup --adb-version latest --adb-sha256 <sha256>

# Run QD-001..QD-009 diagnostics
quest doctor --project-dir my-quest-app
```

### 3. Build, Generate SBOM, and Verify APK Before Store Upload
```pwsh
# Build APK + build-metadata.json + sbom.cdx.json
quest ci --project-dir my-quest-app --output-dir my-quest-app/dist --release-version 1.0.0

# Audit APK against Meta Horizon Store VRC rules (QV-001..QV-006)
quest verify my-quest-app/dist/my-quest-app-1.0.0.apk --project-dir my-quest-app --format html --out scorecard.html
```

### 4. Use with AI Coding Agents (`quest mcp`)
Configure your MCP-compatible IDE/agent to run:
```json
{
  "mcpServers": {
    "questforge": {
      "command": "quest",
      "args": ["mcp", "--workspace-root", "."]
    }
  }
}
```

---

## Part 2: For QuestForge Contributors

### Toolchain Requirements
- Rust `1.100.0` (`rust-toolchain.toml`)
- `swartch` CLI (Architecture 4.0 enforcement)
- Bun `1.4.2` + `mweb` CLI (`machineweb`) + `csslense`

### Running All Verification Gates
```pwsh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
swartch --vocabulary config/vocabulary.toml crates/questforge-api crates/questforge-ci crates/questforge-config crates/questforge-toolchain crates/questforge-verify crates/questforge-doctor crates/questforge-init crates/questforge-mcp crates/questforge-ops crates/questforge-server apps/quest
```
