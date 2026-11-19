# QuestForge XR DevOps Agent

Autonomous XR build, verification, Horizon Store, and fleet operations agent powered by `quest mcp` (16 JSON-RPC 2.0 tools) and `quest agent --role <role>` (`studio`, `enterprise`, `platform`, `qa`, `developer`).

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
2. **Toolchain & Sub-Second Verification:** Call `setup_toolchain` (`adb`, `cmdline-tools`, `ovr-platform-util`), `check_source` (`QF-SPEC-001`, `QF-SYNC-001`, `QF-VRC-001`, `QF-OXR-001`, `QF-PERF-001`), and `doctor_check` (`fix: true` to auto-heal config drift).
3. **Build & Store Readiness:** Call `build_project` (`quest ci`), then `verify_release` (`QV-001`–`QV-010`, including `QV-009` backup security and `QV-010` store assets) and require `score == 100` (`ready == true`) before release.
4. **Failure Triage:** On build errors, call `analyze_build_failure` with the failing log and `search_docs` for remediation steps.
5. **Horizon Store, Fleet & MDM Distribution:** Call `ovr_store` (1:1 `ovr-platform-util` actions) or `upload_store` for Meta Horizon Store channels (`ALPHA`, `BETA`, `RC`, `PRODUCTION`), promote builds across channels (`set-release-channel-build` or `quest agent --goal promote`), invite testers (`manage_channel_testers`), connect untethered headsets (`fleet_connect`), discover headsets (`fleet_discover`), and publish to MDM (`publish_mdm`).
