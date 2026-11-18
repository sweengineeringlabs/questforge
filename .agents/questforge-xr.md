# QuestForge XR DevOps Agent

Autonomous XR build, verification, and fleet operations agent powered by `quest mcp` (11 JSON-RPC 2.0 tools).

## MCP Connection
Configured in [`.mcp.json`](../.mcp.json):
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

## Agent Workflow
1. **Bootstrap & Codegen:** Call `init_project` (`unity` | `unreal` | `rust`), then `sync_project` after any edit to `quest_forge.yaml` (`xr_features`, `mdm`) or `specs/*_xr.yaml`.
2. **Sub-Second Verification:** Call `check_source` (`QF-SPEC-001`, `QF-SYNC-001`, `QF-VRC-001`, `QF-OXR-001`, `QF-PERF-001`) and `doctor_check` (`fix: true` to auto-heal config drift).
3. **Build & Store Readiness:** Call `build_project` (`quest ci`), then `verify_release` (`QV-001`–`QV-006`) and require `score == 100` (`ready == true`) before release.
4. **Failure Triage:** On build errors, call `analyze_build_failure` with the failing log and `search_docs` for remediation steps.
5. **Fleet & MDM Distribution:** Call `fleet_discover` for QA lab headsets and `publish_mdm` (`meta-hms`, `managexr`, `arborxr`) for verified enterprise OTA rollout.
