//! QuestForge Model Context Protocol (MCP) server, tool schemas, and dispatch.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use questforge_ci::{CiOptions, CiOrchestrator, CiProvider};
use questforge_config::EngineKind;
use questforge_doctor::{DoctorOptions, DoctorRunner};
use questforge_init::{InitOptions, ProjectScaffolder};
use questforge_ops::{
    FleetInventory, FleetManager, HmsReleaseRequest, MdmProvider, OpsEnv, OvrStoreAction,
    OvrStoreRequest, StoreUploadRequest,
};
use questforge_toolchain::{SetupOptions, ToolchainProvisioner};
use questforge_verify::{ApkVerifier, VerifyOptions};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::McpError;

const MCP_PROTOCOL_VERSION: &str = "2024-11-05";

/// Typed MCP tool metadata exposed via `tools/list`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpToolDefinition {
    /// Tool name.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// JSON Schema for tool arguments.
    pub input_schema: Value,
}

/// JSON-RPC 2.0 request envelope.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    /// Protocol version (`"2.0"`).
    pub jsonrpc: String,
    /// Request identifier (`None` for notifications).
    #[serde(default)]
    pub id: Option<Value>,
    /// Method name.
    pub method: String,
    /// Optional parameters object.
    #[serde(default)]
    pub params: Option<Value>,
}

/// JSON-RPC 2.0 response envelope.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    /// Protocol version (`"2.0"`).
    pub jsonrpc: String,
    /// Matching request identifier.
    pub id: Value,
    /// Result payload on success.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    /// Error payload on failure.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC 2.0 error object.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// Numeric error code.
    pub code: i32,
    /// Error summary.
    pub message: String,
}

/// QuestForge MCP server handling `initialize`, `tools/list`, and `tools/call`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuestMcpServer {
    /// Workspace root used for relative documentation searches.
    pub workspace_root: PathBuf,
}

impl Default for QuestMcpServer {
    fn default() -> Self {
        Self {
            workspace_root: PathBuf::from("."),
        }
    }
}

impl QuestMcpServer {
    /// Creates a new `QuestMcpServer` rooted at `workspace_root`.
    #[must_use]
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }

    /// Returns the catalogue of MCP tools exposed by QuestForge.
    #[must_use]
    pub fn tool_definitions() -> Vec<McpToolDefinition> {
        vec![
            McpToolDefinition {
                name: "doctor_check".to_string(),
                description: "Validate Quest developer workstation or CI environment against quest_forge.yaml (QD-001..QD-009).".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "project_dir": { "type": "string", "default": "." },
                        "ci": { "type": "boolean", "default": false },
                        "strict": { "type": "boolean", "default": false },
                        "fix": { "type": "boolean", "default": false }
                    }
                }),
            },
            McpToolDefinition {
                name: "sync_project".to_string(),
                description: "Synchronize quest_forge.yaml xr_features and SDK settings into AndroidManifest.xml, bootstrap code, and engine manifests.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "project_dir": { "type": "string", "default": "." }
                    }
                }),
            },
            McpToolDefinition {
                name: "check_source".to_string(),
                description: "Run sub-second static source checks (QF-SYNC-001, QF-VRC-001, QF-PERF-001) on a QuestForge project.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "project_dir": { "type": "string", "default": "." }
                    }
                }),
            },
            McpToolDefinition {
                name: "init_project".to_string(),
                description: "Scaffold a Quest-ready Unity or Unreal repository with quest_forge.yaml and governance files.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "required": ["target_dir", "name", "package_id", "engine"],
                    "properties": {
                        "target_dir": { "type": "string" },
                        "name": { "type": "string" },
                        "package_id": { "type": "string" },
                        "engine": { "type": "string", "enum": ["unity", "unreal", "rust"] },
                        "engine_version": { "type": "string" },
                        "force": { "type": "boolean", "default": false }
                    }
                }),
            },
            McpToolDefinition {
                name: "build_project".to_string(),
                description: "Run or plan Quest CI build, metadata, SBOM, dependency scan, and optional Meta Horizon Store upload (`ovr-platform-util`).".to_string(),
                input_schema: json!({
                    "type": "object",
                    "required": ["project_dir"],
                    "properties": {
                        "project_dir": { "type": "string" },
                        "output_dir": { "type": "string", "default": "dist" },
                        "release_version": { "type": "string", "default": "0.1.0" },
                        "upload_channel": { "type": "string" },
                        "app_id": { "type": "string" },
                        "dry_run": { "type": "boolean", "default": true }
                    }
                }),
            },
            McpToolDefinition {
                name: "verify_release".to_string(),
                description: "Statically verify an APK for signatures, ARM64 ABI, and Meta VRC policy compliance.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "required": ["apk_path"],
                    "properties": {
                        "apk_path": { "type": "string" },
                        "project_dir": { "type": "string" }
                    }
                }),
            },
            McpToolDefinition {
                name: "generate_pipeline".to_string(),
                description: "Generate a starter CI pipeline template for GitHub Actions, Azure DevOps, or Jenkins.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "required": ["provider"],
                    "properties": {
                        "provider": { "type": "string", "enum": ["github", "azure", "jenkins"] }
                    }
                }),
            },
            McpToolDefinition {
                name: "analyze_build_failure".to_string(),
                description: "Analyze Unity, Gradle, Android NDK, or IL2CPP build logs and suggest root-cause remediations.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "required": ["log"],
                    "properties": {
                        "log": { "type": "string" }
                    }
                }),
            },
            McpToolDefinition {
                name: "search_docs".to_string(),
                description: "Search QuestForge and Meta Quest DevOps documentation by keyword.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "required": ["query"],
                    "properties": {
                        "query": { "type": "string" }
                    }
                }),
            },
            McpToolDefinition {
                name: "fleet_discover".to_string(),
                description: "Discover connected Meta Quest headsets or Android emulators over adb and update fleet inventory.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "group": { "type": "string", "default": "default" },
                        "registry": { "type": "string", "default": ".questforge/fleet.json" }
                    }
                }),
            },
            McpToolDefinition {
                name: "publish_mdm".to_string(),
                description: "Publish a verified APK to Meta HMS, ManageXR, or ArborXR enterprise MDM.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "required": ["app_id", "apk_path"],
                    "properties": {
                        "provider": { "type": "string", "enum": ["meta-hms", "managexr", "arborxr"], "default": "meta-hms" },
                        "app_id": { "type": "string" },
                        "apk_path": { "type": "string" },
                        "channel": { "type": "string", "default": "PRODUCTION" },
                        "device_group": { "type": "string", "default": "default" },
                        "endpoint": { "type": "string" },
                        "token_env": { "type": "string", "default": "QUESTFORGE_MDM_TOKEN" }
                    }
                }),
            },
            McpToolDefinition {
                name: "setup_toolchain".to_string(),
                description: "Provision and SHA-256-verify MetaVR CLI binaries (adb, cmdline-tools, ovr-platform-util) into ~/.questforge/cache.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "project_dir": { "type": "string", "default": "." },
                        "adb_version": { "type": "string" },
                        "adb_sha256": { "type": "string" },
                        "cmdline_tools_version": { "type": "string" },
                        "cmdline_tools_sha256": { "type": "string" },
                        "ovr_version": { "type": "string" },
                        "ovr_url": { "type": "string" },
                        "ovr_sha256": { "type": "string" },
                        "allow_unverified": { "type": "boolean", "default": false }
                    }
                }),
            },
            McpToolDefinition {
                name: "upload_store".to_string(),
                description: "Upload a signed Quest APK directly to a Meta Horizon Store release channel via ovr-platform-util.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "required": ["app_id", "apk_path"],
                    "properties": {
                        "app_id": { "type": "string" },
                        "apk_path": { "type": "string" },
                        "channel": { "type": "string", "default": "ALPHA" },
                        "app_secret_env": { "type": "string", "default": "QUEST_META_APP_SECRET" },
                        "notes": { "type": "string" },
                        "ovr_bin": { "type": "string" }
                    }
                }),
            },
            McpToolDefinition {
                name: "ovr_store".to_string(),
                description: "Execute any 1:1 ovr-platform-util Horizon Store operation (upload-quest-build, download-quest-build, get-release-channel-data, set-release-channel-build, download-asset-file, version).".to_string(),
                input_schema: json!({
                    "type": "object",
                    "required": ["action", "app_id"],
                    "properties": {
                        "action": { "type": "string", "enum": ["upload-quest-build", "download-quest-build", "get-release-channel-data", "set-release-channel-build", "download-asset-file", "version"] },
                        "app_id": { "type": "string" },
                        "apk_path": { "type": "string" },
                        "channel": { "type": "string" },
                        "build_id": { "type": "string" },
                        "dir_path": { "type": "string" },
                        "dlc_sku": { "type": "string" },
                        "notes": { "type": "string" },
                        "app_secret_env": { "type": "string", "default": "QUEST_META_APP_SECRET" },
                        "ovr_bin": { "type": "string" }
                    }
                }),
            },
        ]
    }

    /// Runs the JSON-RPC 2.0 MCP server over stdin/stdout until EOF.
    pub fn serve_stdio(&self) -> Result<(), McpError> {
        let stdin = std::io::stdin();
        let stdout = std::io::stdout();
        let mut reader = BufReader::new(stdin.lock());
        let mut writer = stdout.lock();
        let mut line = String::new();

        loop {
            line.clear();
            let n = reader.read_line(&mut line)?;
            if n == 0 {
                break;
            }
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let resp = match serde_json::from_str::<JsonRpcRequest>(trimmed) {
                Ok(req) => self.handle_request(&req),
                Err(err) => Some(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: Value::Null,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32700,
                        message: format!("parse error: {err}"),
                    }),
                }),
            };

            if let Some(r) = resp {
                let encoded = serde_json::to_string(&r)?;
                writeln!(writer, "{encoded}")?;
                writer.flush()?;
            }
        }
        Ok(())
    }

    /// Processes a single JSON-RPC 2.0 request and returns an optional response (`None` for notifications).
    #[must_use]
    pub fn handle_request(&self, req: &JsonRpcRequest) -> Option<JsonRpcResponse> {
        let id = req.id.clone()?;
        let response = match req.method.as_str() {
            "initialize" => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(json!({
                    "protocolVersion": MCP_PROTOCOL_VERSION,
                    "capabilities": {
                        "tools": {}
                    },
                    "serverInfo": {
                        "name": "quest-mcp",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                })),
                error: None,
            },
            "ping" => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(json!({})),
                error: None,
            },
            "tools/list" => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(json!({
                    "tools": Self::tool_definitions()
                })),
                error: None,
            },
            "tools/call" => self.handle_tool_call(id, req.params.as_ref()),
            other => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32601,
                    message: format!("method not found: {other}"),
                }),
            },
        };
        Some(response)
    }

    fn handle_tool_call(&self, id: Value, params: Option<&Value>) -> JsonRpcResponse {
        let Some(params_obj) = params else {
            return invalid_params(id, "missing params");
        };
        let Some(name) = params_obj.get("name").and_then(|v| v.as_str()) else {
            return invalid_params(id, "missing tool name");
        };
        let args = params_obj
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| json!({}));

        let outcome = match name {
            "doctor_check" => call_doctor(&self.workspace_root, &args),
            "sync_project" => call_sync(&self.workspace_root, &args),
            "check_source" => call_check_source(&self.workspace_root, &args),
            "init_project" => call_init(&self.workspace_root, &args),
            "build_project" => call_build(&self.workspace_root, &args),
            "verify_release" => call_verify(&self.workspace_root, &args),
            "generate_pipeline" => call_generate_pipeline(&args),
            "analyze_build_failure" => call_analyze_build_failure(&args),
            "search_docs" => self.call_search_docs(&args),
            "fleet_discover" => call_fleet_discover(&self.workspace_root, &args),
            "publish_mdm" => call_publish_mdm(&self.workspace_root, &args),
            "setup_toolchain" => call_setup_toolchain(&self.workspace_root, &args),
            "upload_store" => call_upload_store(&self.workspace_root, &args),
            "ovr_store" => call_ovr_store(&self.workspace_root, &args),
            unknown => {
                return JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32602,
                        message: format!("unknown tool: {unknown}"),
                    }),
                };
            }
        };

        match outcome {
            Ok(payload) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(json!({
                    "content": [{
                        "type": "text",
                        "text": payload
                    }],
                    "isError": false
                })),
                error: None,
            },
            Err(err_msg) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(json!({
                    "content": [{
                        "type": "text",
                        "text": err_msg
                    }],
                    "isError": true
                })),
                error: None,
            },
        }
    }

    fn call_search_docs(&self, args: &Value) -> Result<String, String> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|q| !q.is_empty())
            .ok_or_else(|| "missing non-empty 'query' argument".to_string())?;
        let needle = query.to_ascii_lowercase();
        let mut hits = Vec::new();
        let docs_dir = self.workspace_root.join("docs");
        if docs_dir.exists() {
            collect_doc_hits(&docs_dir, &needle, &mut hits);
        }
        if hits.is_empty() {
            hits.push(json!({
                "file": "built-in",
                "line": 1,
                "snippet": format!("No local docs matched '{query}'. Try `quest doctor --help`, `quest verify --help`, or `quest ci --help`.")
            }));
        }
        serde_json::to_string_pretty(&json!({ "query": query, "matches": hits }))
            .map_err(|e| e.to_string())
    }
}

fn resolve_workspace_path(workspace_root: &Path, raw: &str) -> PathBuf {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "." {
        return workspace_root.to_path_buf();
    }
    let candidate = Path::new(trimmed);
    if candidate.is_absolute() || candidate.exists() {
        return candidate.to_path_buf();
    }
    let joined = workspace_root.join(candidate);
    if joined.exists() {
        return joined;
    }
    if workspace_root
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n == trimmed)
    {
        return workspace_root.to_path_buf();
    }
    joined
}

fn call_doctor(workspace_root: &Path, args: &Value) -> Result<String, String> {
    let raw_dir = args
        .get("project_dir")
        .and_then(|v| v.as_str())
        .unwrap_or(".");
    let project_dir = resolve_workspace_path(workspace_root, raw_dir);
    let ci = args.get("ci").and_then(|v| v.as_bool()).unwrap_or(false);
    let strict = args
        .get("strict")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let fix = args.get("fix").and_then(|v| v.as_bool()).unwrap_or(false);
    if fix {
        let _ = ProjectScaffolder::sync_project(&project_dir);
    }

    let report = DoctorRunner::run(&DoctorOptions {
        project_dir,
        ci,
        strict,
        fix,
    })
    .map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&report).map_err(|e| e.to_string())
}

fn call_sync(workspace_root: &Path, args: &Value) -> Result<String, String> {
    let raw_dir = args
        .get("project_dir")
        .and_then(|v| v.as_str())
        .unwrap_or(".");
    let project_dir = resolve_workspace_path(workspace_root, raw_dir);
    let report = ProjectScaffolder::sync_project(&project_dir).map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&report).map_err(|e| e.to_string())
}

fn call_check_source(workspace_root: &Path, args: &Value) -> Result<String, String> {
    let raw_dir = args
        .get("project_dir")
        .and_then(|v| v.as_str())
        .unwrap_or(".");
    let project_dir = resolve_workspace_path(workspace_root, raw_dir);
    let report = ProjectScaffolder::check_project(&project_dir).map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&report).map_err(|e| e.to_string())
}

fn call_init(workspace_root: &Path, args: &Value) -> Result<String, String> {
    let target_dir_raw = args
        .get("target_dir")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing 'target_dir'".to_string())?;
    let target_dir = resolve_workspace_path(workspace_root, target_dir_raw);
    let name = args
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing 'name'".to_string())?;
    let package_id = args
        .get("package_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing 'package_id'".to_string())?;
    let engine = match args.get("engine").and_then(|v| v.as_str()) {
        Some("unity") => EngineKind::Unity,
        Some("unreal") => EngineKind::Unreal,
        Some("rust") => EngineKind::Rust,
        _ => {
            return Err(
                "invalid or missing 'engine' (expected 'unity', 'unreal', or 'rust')".to_string(),
            );
        }
    };
    let engine_version = args
        .get("engine_version")
        .and_then(|v| v.as_str())
        .map(ToString::to_string);
    let force = args.get("force").and_then(|v| v.as_bool()).unwrap_or(false);

    let report = ProjectScaffolder::scaffold(&InitOptions {
        target_dir,
        name: name.to_string(),
        package_id: package_id.to_string(),
        engine,
        engine_version,
        template_url: None,
        force,
    })
    .map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&report).map_err(|e| e.to_string())
}

fn call_build(workspace_root: &Path, args: &Value) -> Result<String, String> {
    let raw_dir = args
        .get("project_dir")
        .and_then(|v| v.as_str())
        .unwrap_or(".");
    let project_dir = resolve_workspace_path(workspace_root, raw_dir);
    let output_dir_raw = args
        .get("output_dir")
        .and_then(|v| v.as_str())
        .unwrap_or("dist");
    let output_dir = if Path::new(output_dir_raw).is_absolute() {
        PathBuf::from(output_dir_raw)
    } else {
        project_dir.join(output_dir_raw)
    };
    let release_version = args
        .get("release_version")
        .and_then(|v| v.as_str())
        .unwrap_or("0.1.0");
    let upload_channel = args
        .get("upload_channel")
        .and_then(|v| v.as_str())
        .map(ToString::to_string);
    let app_id = args
        .get("app_id")
        .and_then(|v| v.as_str())
        .map(ToString::to_string);
    let dry_run = args
        .get("dry_run")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let report = CiOrchestrator::run(&CiOptions {
        project_dir,
        output_dir,
        release_version: release_version.to_string(),
        upload_channel,
        app_id,
        dry_run,
    })
    .map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&report).map_err(|e| e.to_string())
}

fn call_verify(workspace_root: &Path, args: &Value) -> Result<String, String> {
    let apk_raw = args
        .get("apk_path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing 'apk_path'".to_string())?;
    let apk_path = resolve_workspace_path(workspace_root, apk_raw);
    let project_dir = Some(
        args.get("project_dir")
            .and_then(|v| v.as_str())
            .map(|raw| resolve_workspace_path(workspace_root, raw))
            .unwrap_or_else(|| workspace_root.to_path_buf()),
    );

    let report = ApkVerifier::verify(&VerifyOptions {
        apk_path,
        project_dir,
        max_apk_bytes: None,
    })
    .map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&report).map_err(|e| e.to_string())
}

fn call_generate_pipeline(args: &Value) -> Result<String, String> {
    let provider = match args.get("provider").and_then(|v| v.as_str()) {
        Some("github") => CiProvider::Github,
        Some("azure") => CiProvider::Azure,
        Some("jenkins") => CiProvider::Jenkins,
        _ => {
            return Err(
                "invalid or missing 'provider' (expected 'github', 'azure', or 'jenkins')"
                    .to_string(),
            );
        }
    };
    Ok(CiOrchestrator::render_pipeline_template(provider).to_string())
}

fn call_analyze_build_failure(args: &Value) -> Result<String, String> {
    let log = args
        .get("log")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing 'log'".to_string())?;
    let lower = log.to_ascii_lowercase();
    let mut diagnostics = Vec::new();

    if lower.contains("ndk") || lower.contains("il2cpp") {
        diagnostics.push(json!({
            "category": "android-ndk-il2cpp",
            "remediation": "Run `quest doctor` to verify ANDROID_NDK_HOME and Unity IL2CPP module."
        }));
    }
    if lower.contains("gradle") || lower.contains("sdk") || lower.contains("build-tools") {
        diagnostics.push(json!({
            "category": "android-gradle-sdk",
            "remediation": "Verify target_sdk and android_build_tools in quest_forge.yaml using `quest doctor`."
        }));
    }
    if lower.contains("keystore") || lower.contains("signature") {
        diagnostics.push(json!({
            "category": "apk-signing",
            "remediation": "Ensure QUEST_KEYSTORE_PATH and QUEST_KEY_ALIAS are set and run `quest verify`."
        }));
    }
    if diagnostics.is_empty() {
        diagnostics.push(json!({
            "category": "general",
            "remediation": "Run `quest doctor --ci` to check local toolchain state."
        }));
    }

    serde_json::to_string_pretty(&json!({ "diagnostics": diagnostics })).map_err(|e| e.to_string())
}

fn call_fleet_discover(workspace_root: &Path, args: &Value) -> Result<String, String> {
    let group = args
        .get("group")
        .and_then(|v| v.as_str())
        .unwrap_or("default");
    let registry_raw = args
        .get("registry")
        .and_then(|v| v.as_str())
        .unwrap_or(".questforge/fleet.json");
    let reg_path = resolve_workspace_path(workspace_root, registry_raw);
    let mut inv = FleetInventory::load(&reg_path).map_err(|e| e.to_string())?;
    let devs =
        FleetManager::discover(&mut inv, group, &OpsEnv::System).map_err(|e| e.to_string())?;
    inv.save(&reg_path).map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&devs).map_err(|e| e.to_string())
}

fn call_publish_mdm(workspace_root: &Path, args: &Value) -> Result<String, String> {
    let provider = match args.get("provider").and_then(|v| v.as_str()) {
        None | Some("meta-hms") => MdmProvider::MetaHms,
        Some("managexr") => MdmProvider::ManageXr,
        Some("arborxr") => MdmProvider::ArborXr,
        Some(other) => return Err(format!("unknown MDM provider: '{other}'")),
    };
    let app_id = args
        .get("app_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing 'app_id'".to_string())?
        .to_string();
    let apk_raw = args
        .get("apk_path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing 'apk_path'".to_string())?;
    let apk_path = resolve_workspace_path(workspace_root, apk_raw);
    let channel = args
        .get("channel")
        .and_then(|v| v.as_str())
        .unwrap_or("PRODUCTION")
        .to_string();
    let device_group = args
        .get("device_group")
        .and_then(|v| v.as_str())
        .unwrap_or("default")
        .to_string();
    let endpoint = args
        .get("endpoint")
        .and_then(|v| v.as_str())
        .map(ToString::to_string);
    let token_env = args
        .get("token_env")
        .and_then(|v| v.as_str())
        .unwrap_or("QUESTFORGE_MDM_TOKEN");
    let access_token = std::env::var(token_env).unwrap_or_default();

    let receipt = FleetManager::publish_to_mdm(
        &HmsReleaseRequest {
            provider,
            app_id,
            apk_path,
            channel,
            device_group,
            endpoint,
            access_token,
        },
        &OpsEnv::System,
    )
    .map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&receipt).map_err(|e| e.to_string())
}

fn call_setup_toolchain(workspace_root: &Path, args: &Value) -> Result<String, String> {
    let project_dir = Some(
        args.get("project_dir")
            .and_then(|v| v.as_str())
            .map(|raw| resolve_workspace_path(workspace_root, raw))
            .unwrap_or_else(|| workspace_root.to_path_buf()),
    );
    let allow_unverified = args
        .get("allow_unverified")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let cmdline_tools_version = args
        .get("cmdline_tools_version")
        .and_then(|v| v.as_str())
        .map(ToString::to_string);

    let report = ToolchainProvisioner::provision(&SetupOptions {
        project_dir,
        adb_version: args
            .get("adb_version")
            .and_then(|v| v.as_str())
            .map(ToString::to_string),
        adb_sha256: args
            .get("adb_sha256")
            .and_then(|v| v.as_str())
            .map(ToString::to_string),
        include_cmdline_tools: cmdline_tools_version.is_some(),
        cmdline_tools_version,
        cmdline_tools_sha256: args
            .get("cmdline_tools_sha256")
            .and_then(|v| v.as_str())
            .map(ToString::to_string),
        ovr_version: args
            .get("ovr_version")
            .and_then(|v| v.as_str())
            .map(ToString::to_string),
        ovr_url: args
            .get("ovr_url")
            .and_then(|v| v.as_str())
            .map(ToString::to_string),
        ovr_sha256: args
            .get("ovr_sha256")
            .and_then(|v| v.as_str())
            .map(ToString::to_string),
        cache_dir: None,
        require_checksum: !allow_unverified,
    })
    .map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&report).map_err(|e| e.to_string())
}

fn call_upload_store(workspace_root: &Path, args: &Value) -> Result<String, String> {
    let app_id = args
        .get("app_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing app_id".to_string())?
        .to_string();
    let apk_raw = args
        .get("apk_path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing apk_path".to_string())?;
    let apk_path = resolve_workspace_path(workspace_root, apk_raw);
    let channel = args
        .get("channel")
        .and_then(|v| v.as_str())
        .unwrap_or("ALPHA")
        .to_string();
    let secret_env = args
        .get("app_secret_env")
        .and_then(|v| v.as_str())
        .unwrap_or("QUEST_META_APP_SECRET");
    let app_secret = std::env::var(secret_env).unwrap_or_default();
    let release_notes = args
        .get("notes")
        .and_then(|v| v.as_str())
        .map(ToString::to_string);
    let ovr_bin = args
        .get("ovr_bin")
        .and_then(|v| v.as_str())
        .map(PathBuf::from);

    let receipt = FleetManager::upload_to_store(
        &StoreUploadRequest {
            app_id,
            app_secret,
            apk_path,
            channel,
            release_notes,
            ovr_bin,
        },
        &OpsEnv::System,
    )
    .map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&receipt).map_err(|e| e.to_string())
}

fn call_ovr_store(workspace_root: &Path, args: &Value) -> Result<String, String> {
    let action_raw = args
        .get("action")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing action".to_string())?;
    let action = match action_raw {
        "upload-quest-build" => OvrStoreAction::UploadQuestBuild,
        "download-quest-build" => OvrStoreAction::DownloadQuestBuild,
        "get-release-channel-data" => OvrStoreAction::GetReleaseChannelData,
        "set-release-channel-build" => OvrStoreAction::SetReleaseChannelBuild,
        "download-asset-file" => OvrStoreAction::DownloadAssetFile,
        "version" => OvrStoreAction::Version,
        other => return Err(format!("unsupported ovr_store action: {other}")),
    };
    let app_id = args
        .get("app_id")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let secret_env = args
        .get("app_secret_env")
        .and_then(|v| v.as_str())
        .unwrap_or("QUEST_META_APP_SECRET");
    let app_secret = std::env::var(secret_env).unwrap_or_default();

    let receipt = FleetManager::execute_ovr_store(
        &OvrStoreRequest {
            action,
            app_id,
            app_secret,
            apk_path: args
                .get("apk_path")
                .and_then(|v| v.as_str())
                .map(|raw| resolve_workspace_path(workspace_root, raw)),
            channel: args
                .get("channel")
                .and_then(|v| v.as_str())
                .map(ToString::to_string),
            build_id: args
                .get("build_id")
                .and_then(|v| v.as_str())
                .map(ToString::to_string),
            dir_path: args
                .get("dir_path")
                .and_then(|v| v.as_str())
                .map(|raw| resolve_workspace_path(workspace_root, raw)),
            dlc_sku: args
                .get("dlc_sku")
                .and_then(|v| v.as_str())
                .map(ToString::to_string),
            release_notes: args
                .get("notes")
                .and_then(|v| v.as_str())
                .map(ToString::to_string),
            ovr_bin: args
                .get("ovr_bin")
                .and_then(|v| v.as_str())
                .map(PathBuf::from),
        },
        &OpsEnv::System,
    )
    .map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&receipt).map_err(|e| e.to_string())
}

fn collect_doc_hits(dir: &Path, needle: &str, hits: &mut Vec<Value>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_doc_hits(&path, needle, hits);
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            if let Ok(content) = fs::read_to_string(&path) {
                for (idx, line) in content.lines().enumerate() {
                    if line.to_ascii_lowercase().contains(needle) {
                        hits.push(json!({
                            "file": path.display().to_string(),
                            "line": idx + 1,
                            "snippet": line.trim()
                        }));
                        if hits.len() >= 10 {
                            return;
                        }
                    }
                }
            }
        }
    }
}

fn invalid_params(id: Value, message: &str) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: None,
        error: Some(JsonRpcError {
            code: -32602,
            message: message.to_string(),
        }),
    }
}
