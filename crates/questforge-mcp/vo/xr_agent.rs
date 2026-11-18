//! Autonomous QuestForge XR DevOps Agent (MCP JSON-RPC 2.0 client + self-healing state machine).

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::spi::AutoLlmCredentialResolver;
use crate::traits::LlmProviderSpi;
use crate::vo::llm_providers::{LlmChatMessage, LlmEndpointConfig, LlmToolSchema};
use crate::vo::mcp_server::{JsonRpcRequest, JsonRpcResponse, QuestMcpServer};
use crate::McpError;

/// Autonomous goal executed by [`QuestXrAgent`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentGoal {
    /// Validate and auto-heal source specs, sync drift, VRC, OpenXR parity, and perf.
    Check,
    /// Check + build APK/SBOM + verify 100/100 Meta VRC readiness.
    #[default]
    Release,
    /// Release + discover connected QA lab headsets via `fleet_discover`.
    FleetRelease,
}

/// Target audience profile for [`QuestXrAgent`] workflows (`docs/0-ideation/concept.md`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentAudience {
    /// XR Game Studios (Unity & Unreal Teams): scaffolding, SDK doctor, VRC & release builds.
    Studio,
    /// Enterprise XR Training & Simulation Teams: supply-chain SBOM, auditability & lab rollout.
    Enterprise,
    /// Platform & DevOps Engineers: CI pipelines, `quest_forge.yaml` sync & build diagnostics.
    Platform,
    /// XR QA & Release Managers: VRC scorecards, fleet discovery, deploy, logcat & screenshots.
    Qa,
    /// AI-Native XR Developers: full MCP toolset for local coding & debugging.
    #[default]
    Developer,
}

impl AgentAudience {
    /// Default deterministic [`AgentGoal`] for this audience.
    #[must_use]
    pub const fn default_goal(self) -> AgentGoal {
        match self {
            Self::Studio => AgentGoal::Release,
            Self::Enterprise | Self::Qa => AgentGoal::FleetRelease,
            Self::Platform | Self::Developer => AgentGoal::Check,
        }
    }

    /// Tailored system prompt for this audience's workflow.
    #[must_use]
    pub const fn system_prompt(self) -> &'static str {
        match self {
            Self::Studio => "You are the QuestForge XR Studio Agent for Unity/Unreal game teams. Use MCP tools (`init_project`, `setup_toolchain`, `doctor_check`, `check_source`, `sync_project`, `build_project`, `verify_release`) to keep projects VRC-ready and shippable.",
            Self::Enterprise => "You are the QuestForge Enterprise XR Agent for regulated training and simulation teams. Prioritize supply-chain verification, SBOM/release auditability, and controlled MDM/lab rollout (`verify_release`, `fleet_discover`, `publish_mdm`) using MCP tools.",
            Self::Platform => "You are the QuestForge Platform & DevOps Agent. Focus on environment parity (`setup_toolchain`, `doctor_check`), CI pipeline generation (`generate_pipeline`), static drift (`check_source`, `sync_project`), and build diagnostics (`analyze_build_failure`).",
            Self::Qa => "You are the QuestForge QA & Release Manager Agent. Focus on release readiness (`verify_release`, `check_source`), lab headset discovery (`fleet_discover`), and release distribution (`publish_mdm`).",
            Self::Developer => "You are the QuestForge XR Developer Agent. Use the provided MCP tools to inspect, sync, diagnose, build, verify, or deploy Quest projects.",
        }
    }

    /// Preferred MCP tool names prioritized for this audience (`Developer` includes all tools).
    #[must_use]
    pub const fn preferred_tools(self) -> &'static [&'static str] {
        match self {
            Self::Studio => &[
                "init_project",
                "setup_toolchain",
                "doctor_check",
                "check_source",
                "sync_project",
                "build_project",
                "verify_release",
            ],
            Self::Enterprise => &[
                "doctor_check",
                "check_source",
                "build_project",
                "verify_release",
                "fleet_discover",
                "publish_mdm",
                "search_docs",
            ],
            Self::Platform => &[
                "setup_toolchain",
                "doctor_check",
                "check_source",
                "sync_project",
                "generate_pipeline",
                "analyze_build_failure",
                "search_docs",
            ],
            Self::Qa => &[
                "verify_release",
                "fleet_discover",
                "publish_mdm",
                "upload_store",
                "check_source",
                "search_docs",
            ],
            Self::Developer => &[],
        }
    }
}

/// Recorded step from an autonomous agent execution.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentStepLog {
    /// Step sequence number (1-indexed).
    pub step: usize,
    /// MCP tool invoked (e.g. `check_source`, `sync_project`).
    pub tool: String,
    /// True if the tool call succeeded without `isError`.
    pub ok: bool,
    /// Concise summary of the step outcome.
    pub summary: String,
}

/// Configuration for running [`QuestXrAgent`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentOptions {
    /// Target QuestForge project directory.
    pub project_dir: PathBuf,
    /// Build output directory.
    pub output_dir: PathBuf,
    /// Release version string.
    pub release_version: String,
    /// Autonomous goal to achieve.
    pub goal: AgentGoal,
    /// If true, passes `dry_run: true` to `build_project` and skips `verify_release` if no APK exists.
    pub dry_run: bool,
}

/// Final report emitted by [`QuestXrAgent`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentRunReport {
    /// Goal executed.
    pub goal: AgentGoal,
    /// MCP tool names discovered from `tools/list`.
    pub discovered_tools: Vec<String>,
    /// Ordered trace of autonomous tool calls and self-healing actions.
    pub steps: Vec<AgentStepLog>,
    /// Final VRC verification score if `verify_release` ran.
    pub verify_score: Option<u8>,
    /// True if the goal completed with all gates passing.
    pub passed: bool,
}

/// Configuration for running [`QuestXrAgent`] in natural-language LLM ReAct mode.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LlmAgentOptions {
    /// Workspace root passed to `quest mcp`.
    pub project_dir: PathBuf,
    /// Natural-language task prompt for the LLM agent.
    pub prompt: String,
    /// OpenAI-compatible chat completions endpoint (e.g. OpenAI, Ollama, Gemini).
    pub llm_url: String,
    /// Model identifier (e.g. `gpt-4o-mini`, `llama3.2`).
    pub model: String,
    /// Optional Bearer API key (may be empty for local Ollama).
    pub api_key: String,
    /// Maximum ReAct reasoning turns before stopping.
    pub max_turns: usize,
}

/// Result emitted by an LLM-driven ReAct session routed through `quest mcp`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LlmRunReport {
    /// Model used.
    pub model: String,
    /// MCP tool names discovered from `tools/list`.
    pub discovered_tools: Vec<String>,
    /// Tool calls executed through `quest mcp`.
    pub steps: Vec<AgentStepLog>,
    /// Final natural-language response from the LLM.
    pub final_answer: String,
}

/// Real coded autonomous XR DevOps agent that communicates with `quest mcp` over JSON-RPC 2.0.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct QuestXrAgent;

impl QuestXrAgent {
    /// Spawns `<quest_bin> mcp --workspace-root <project_dir>` as a child process and runs the
    /// autonomous agent loop over live stdio JSON-RPC 2.0 pipes.
    pub fn run_stdio_client(
        quest_bin: &Path,
        opts: &AgentOptions,
    ) -> Result<AgentRunReport, McpError> {
        let mut child = Command::new(quest_bin)
            .args(["mcp", "--workspace-root"])
            .arg(&opts.project_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;

        let mut transport = StdioRpcTransport::from_child(&mut child)?;
        let res = Self::execute_loop(&mut transport, opts);
        drop(transport);
        let _ = child.wait();
        res
    }

    /// Runs the autonomous agent loop directly against an in-process [`QuestMcpServer`].
    pub fn run_in_process(opts: &AgentOptions) -> Result<AgentRunReport, McpError> {
        let server = QuestMcpServer::new(opts.project_dir.clone());
        let mut transport = InProcessRpcTransport { server, next_id: 1 };
        Self::execute_loop(&mut transport, opts)
    }

    /// Spawns `<quest_bin> mcp --workspace-root <project_dir>` and runs an LLM ReAct loop,
    /// routing every LLM `tool_call` through `quest mcp` over JSON-RPC 2.0.
    pub fn run_llm_stdio_client(
        quest_bin: &Path,
        opts: &LlmAgentOptions,
    ) -> Result<LlmRunReport, McpError> {
        let provider = AutoLlmCredentialResolver::provider_for_url(&opts.llm_url);
        let auth_source = if opts.api_key.starts_with("sk-ant-oat") {
            "oauth".to_string()
        } else {
            "api-key".to_string()
        };
        let config = LlmEndpointConfig {
            provider_id: provider.provider_id().to_string(),
            endpoint_url: opts.llm_url.clone(),
            default_model: opts.model.clone(),
            api_key: opts.api_key.clone(),
            auth_source,
        };
        Self::run_llm_stdio_client_with_spi(
            quest_bin,
            &opts.project_dir,
            &opts.prompt,
            opts.max_turns,
            provider,
            &config,
        )
    }

    /// Spawns `<quest_bin> mcp --workspace-root <project_dir>` and runs an LLM ReAct loop
    /// using an explicit [`LlmProviderSpi`] implementation and [`LlmEndpointConfig`].
    pub fn run_llm_stdio_client_with_spi(
        quest_bin: &Path,
        project_dir: &Path,
        prompt: &str,
        max_turns: usize,
        provider: &dyn LlmProviderSpi,
        config: &LlmEndpointConfig,
    ) -> Result<LlmRunReport, McpError> {
        Self::run_llm_stdio_client_with_spi_and_audience(
            quest_bin,
            project_dir,
            prompt,
            max_turns,
            provider,
            config,
            AgentAudience::Developer,
        )
    }

    /// Spawns `<quest_bin> mcp --workspace-root <project_dir>` and runs an audience-tailored LLM ReAct loop.
    pub fn run_llm_stdio_client_with_spi_and_audience(
        quest_bin: &Path,
        project_dir: &Path,
        prompt: &str,
        max_turns: usize,
        provider: &dyn LlmProviderSpi,
        config: &LlmEndpointConfig,
        audience: AgentAudience,
    ) -> Result<LlmRunReport, McpError> {
        let mut child = Command::new(quest_bin)
            .args(["mcp", "--workspace-root"])
            .arg(project_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;

        let mut transport = StdioRpcTransport::from_child(&mut child)?;
        let res = Self::execute_llm_loop(
            &mut transport,
            prompt,
            max_turns,
            provider,
            config,
            audience,
            |http_req| {
                let mut req = ureq::post(&http_req.url);
                for (header, val) in &http_req.headers {
                    req = req.set(header, val);
                }
                let resp =
                    req.send_string(&http_req.body.to_string())
                        .map_err(|err| match err {
                            ureq::Error::Status(code, r) => {
                                let body = r.into_string().unwrap_or_default();
                                std::io::Error::other(format!("LLM HTTP {code}: {body}"))
                            }
                            other => std::io::Error::other(format!("LLM HTTP error: {other}")),
                        })?;
                let raw = resp
                    .into_string()
                    .map_err(|err| std::io::Error::other(format!("LLM read error: {err}")))?;
                Ok(serde_json::from_str(&raw)?)
            },
        );
        drop(transport);
        let _ = child.wait();
        res
    }

    /// Runs the LLM ReAct loop in-process using a custom chat-completion callback (for tests).
    pub fn run_llm_in_process_with_completer<F>(
        opts: &LlmAgentOptions,
        completer: F,
    ) -> Result<LlmRunReport, McpError>
    where
        F: FnMut(&Value) -> Result<Value, McpError>,
    {
        let provider = AutoLlmCredentialResolver::provider_for_url(&opts.llm_url);
        Self::run_llm_in_process_with_provider(opts, provider, completer)
    }

    /// Runs the LLM ReAct loop in-process with an explicit [`LlmProviderSpi`] and callback.
    pub fn run_llm_in_process_with_provider<F>(
        opts: &LlmAgentOptions,
        provider: &dyn LlmProviderSpi,
        completer: F,
    ) -> Result<LlmRunReport, McpError>
    where
        F: FnMut(&Value) -> Result<Value, McpError>,
    {
        Self::run_llm_in_process_with_audience(opts, provider, AgentAudience::Developer, completer)
    }

    /// Runs the LLM ReAct loop in-process with an explicit [`LlmProviderSpi`], [`AgentAudience`], and callback.
    pub fn run_llm_in_process_with_audience<F>(
        opts: &LlmAgentOptions,
        provider: &dyn LlmProviderSpi,
        audience: AgentAudience,
        mut completer: F,
    ) -> Result<LlmRunReport, McpError>
    where
        F: FnMut(&Value) -> Result<Value, McpError>,
    {
        let server = QuestMcpServer::new(opts.project_dir.clone());
        let mut transport = InProcessRpcTransport { server, next_id: 1 };
        let config = LlmEndpointConfig {
            provider_id: provider.provider_id().to_string(),
            endpoint_url: opts.llm_url.clone(),
            default_model: opts.model.clone(),
            api_key: opts.api_key.clone(),
            auth_source: "test".to_string(),
        };
        Self::execute_llm_loop(
            &mut transport,
            &opts.prompt,
            opts.max_turns,
            provider,
            &config,
            audience,
            |http_req| completer(&http_req.body),
        )
    }

    fn execute_llm_loop<T: RpcTransport, F>(
        rpc: &mut T,
        prompt: &str,
        max_turns: usize,
        provider: &dyn LlmProviderSpi,
        config: &LlmEndpointConfig,
        audience: AgentAudience,
        mut completer: F,
    ) -> Result<LlmRunReport, McpError>
    where
        F: FnMut(&crate::vo::llm_providers::LlmHttpRequest) -> Result<Value, McpError>,
    {
        let _init = rpc.request("initialize", json!({}))?;
        let tools_res = rpc.request("tools/list", json!({}))?;
        let raw_tools = tools_res
            .get("tools")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let preferred = audience.preferred_tools();
        let mut discovered_tools = Vec::with_capacity(raw_tools.len());
        let mut tool_schemas = Vec::with_capacity(raw_tools.len());
        for t in &raw_tools {
            if let Some(name) = t.get("name").and_then(|v| v.as_str()) {
                discovered_tools.push(name.to_string());
                if preferred.is_empty() || preferred.contains(&name) {
                    tool_schemas.push(LlmToolSchema {
                        name: name.to_string(),
                        description: t
                            .get("description")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        input_schema: t
                            .get("inputSchema")
                            .cloned()
                            .unwrap_or_else(|| json!({"type":"object"})),
                    });
                }
            }
        }

        let system_prompt = audience.system_prompt();
        let mut messages = vec![LlmChatMessage::User(prompt.to_string())];
        let mut steps = Vec::new();
        let max_turns = max_turns.max(1);

        for _ in 0..max_turns {
            let http_req =
                provider.build_request(config, system_prompt, &messages, &tool_schemas)?;
            let llm_resp = completer(&http_req)?;
            let turn = provider.parse_response(&llm_resp)?;

            if turn.tool_calls.is_empty() {
                return Ok(LlmRunReport {
                    model: config.default_model.clone(),
                    discovered_tools,
                    steps,
                    final_answer: turn.text,
                });
            }

            messages.push(LlmChatMessage::Assistant {
                text: turn.text,
                tool_calls: turn.tool_calls.clone(),
            });
            for tc in turn.tool_calls {
                let (_ok, tool_out) = call_mcp_tool(rpc, &mut steps, &tc.name, tc.arguments)?;
                messages.push(LlmChatMessage::ToolResult {
                    tool_call_id: tc.id,
                    name: tc.name,
                    content: tool_out.to_string(),
                });
            }
        }

        Ok(LlmRunReport {
            model: config.default_model.clone(),
            discovered_tools,
            steps,
            final_answer: "Reached max ReAct turns before receiving a final LLM summary."
                .to_string(),
        })
    }

    fn execute_loop<T: RpcTransport>(
        rpc: &mut T,
        opts: &AgentOptions,
    ) -> Result<AgentRunReport, McpError> {
        let _init = rpc.request("initialize", json!({}))?;
        let tools_res = rpc.request("tools/list", json!({}))?;
        let discovered_tools = tools_res
            .get("tools")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|t| {
                        t.get("name")
                            .and_then(|n| n.as_str())
                            .map(ToString::to_string)
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let mut steps = Vec::new();
        let proj = opts.project_dir.display().to_string();

        // 1. Sub-second static source & OpenXR check; auto-heal drift via `sync_project`.
        let (check_ok, check_val) = call_mcp_tool(
            rpc,
            &mut steps,
            "check_source",
            json!({ "project_dir": proj }),
        )?;
        let source_passed = check_ok
            && check_val
                .get("passed")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

        if !source_passed {
            let (sync_ok, _) = call_mcp_tool(
                rpc,
                &mut steps,
                "sync_project",
                json!({ "project_dir": opts.project_dir.display().to_string() }),
            )?;
            if !sync_ok {
                return Ok(AgentRunReport {
                    goal: opts.goal,
                    discovered_tools,
                    steps,
                    verify_score: None,
                    passed: false,
                });
            }
            let (recheck_ok, recheck_val) = call_mcp_tool(
                rpc,
                &mut steps,
                "check_source",
                json!({ "project_dir": opts.project_dir.display().to_string() }),
            )?;
            if !recheck_ok
                || !recheck_val
                    .get("passed")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
            {
                return Ok(AgentRunReport {
                    goal: opts.goal,
                    discovered_tools,
                    steps,
                    verify_score: None,
                    passed: false,
                });
            }
        }

        if opts.goal == AgentGoal::Check {
            return Ok(AgentRunReport {
                goal: opts.goal,
                discovered_tools,
                steps,
                verify_score: None,
                passed: true,
            });
        }

        // 2. Build APK, metadata, and CycloneDX 1.5 SBOM via `build_project`.
        let (build_ok, build_val) = call_mcp_tool(
            rpc,
            &mut steps,
            "build_project",
            json!({
                "project_dir": opts.project_dir.display().to_string(),
                "output_dir": opts.output_dir.display().to_string(),
                "release_version": opts.release_version,
                "dry_run": opts.dry_run
            }),
        )?;
        if !build_ok {
            return Ok(AgentRunReport {
                goal: opts.goal,
                discovered_tools,
                steps,
                verify_score: None,
                passed: false,
            });
        }

        // 3. Verify release APK compliance (`QV-001`..`QV-006`) if artifact exists on disk.
        let mut verify_score = None;
        let apk_path_str = build_val
            .get("metadata")
            .and_then(|m| m.get("apk_path"))
            .and_then(|p| p.as_str())
            .map(ToString::to_string);

        if let Some(apk_path) = apk_path_str {
            if Path::new(&apk_path).exists() {
                let (ver_ok, ver_val) = call_mcp_tool(
                    rpc,
                    &mut steps,
                    "verify_release",
                    json!({
                        "apk_path": apk_path,
                        "project_dir": opts.project_dir.display().to_string()
                    }),
                )?;
                let score = ver_val
                    .get("score")
                    .and_then(|v| v.as_u64())
                    .and_then(|n| u8::try_from(n).ok());
                let ready = ver_val
                    .get("ready")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                verify_score = score;
                if !ver_ok || !ready {
                    return Ok(AgentRunReport {
                        goal: opts.goal,
                        discovered_tools,
                        steps,
                        verify_score,
                        passed: false,
                    });
                }
            }
        }

        // 4. Optional QA lab discovery (`fleet_discover`).
        if opts.goal == AgentGoal::FleetRelease {
            let (fleet_ok, _) = call_mcp_tool(
                rpc,
                &mut steps,
                "fleet_discover",
                json!({ "group": "qa-lab" }),
            )?;
            if !fleet_ok {
                return Ok(AgentRunReport {
                    goal: opts.goal,
                    discovered_tools,
                    steps,
                    verify_score,
                    passed: false,
                });
            }
        }

        Ok(AgentRunReport {
            goal: opts.goal,
            discovered_tools,
            steps,
            verify_score,
            passed: true,
        })
    }
}

trait RpcTransport {
    fn request(&mut self, method: &str, params: Value) -> Result<Value, McpError>;
}

struct InProcessRpcTransport {
    server: QuestMcpServer,
    next_id: u64,
}

impl RpcTransport for InProcessRpcTransport {
    fn request(&mut self, method: &str, params: Value) -> Result<Value, McpError> {
        let id = self.next_id;
        self.next_id += 1;
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(id)),
            method: method.to_string(),
            params: Some(params),
        };
        let resp = self
            .server
            .handle_request(&req)
            .ok_or_else(|| std::io::Error::other("missing JSON-RPC response"))?;
        unwrap_rpc_response(resp)
    }
}

struct StdioRpcTransport {
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
}

impl StdioRpcTransport {
    fn from_child(child: &mut Child) -> Result<Self, McpError> {
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| std::io::Error::other("failed to open child stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| std::io::Error::other("failed to open child stdout"))?;
        Ok(Self {
            stdin,
            stdout: BufReader::new(stdout),
            next_id: 1,
        })
    }
}

impl RpcTransport for StdioRpcTransport {
    fn request(&mut self, method: &str, params: Value) -> Result<Value, McpError> {
        let id = self.next_id;
        self.next_id += 1;
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(id)),
            method: method.to_string(),
            params: Some(params),
        };
        let line = serde_json::to_string(&req)?;
        writeln!(self.stdin, "{line}")?;
        self.stdin.flush()?;

        let mut resp_line = String::new();
        let n = self.stdout.read_line(&mut resp_line)?;
        if n == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "MCP server closed stdout unexpectedly",
            )
            .into());
        }
        let resp: JsonRpcResponse = serde_json::from_str(resp_line.trim())?;
        unwrap_rpc_response(resp)
    }
}

fn unwrap_rpc_response(resp: JsonRpcResponse) -> Result<Value, McpError> {
    if let Some(err) = resp.error {
        return Err(
            std::io::Error::other(format!("JSON-RPC error {}: {}", err.code, err.message)).into(),
        );
    }
    Ok(resp.result.unwrap_or_else(|| json!({})))
}

fn call_mcp_tool<T: RpcTransport>(
    rpc: &mut T,
    steps: &mut Vec<AgentStepLog>,
    tool_name: &str,
    arguments: Value,
) -> Result<(bool, Value), McpError> {
    let result = rpc.request(
        "tools/call",
        json!({
            "name": tool_name,
            "arguments": arguments
        }),
    )?;
    let is_error = result
        .get("isError")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let raw_text = result
        .get("content")
        .and_then(|c| c.as_array())
        .and_then(|arr| arr.first())
        .and_then(|item| item.get("text"))
        .and_then(|t| t.as_str())
        .unwrap_or("{}");
    let parsed: Value =
        serde_json::from_str(raw_text).unwrap_or_else(|_| json!({ "raw": raw_text }));

    let summary = if is_error {
        raw_text.lines().next().unwrap_or("error").to_string()
    } else if let Some(passed) = parsed.get("passed").and_then(|v| v.as_bool()) {
        format!("passed={passed}")
    } else if let Some(score) = parsed.get("score").and_then(|v| v.as_u64()) {
        format!("score={score}/100")
    } else {
        "ok".to_string()
    };

    steps.push(AgentStepLog {
        step: steps.len() + 1,
        tool: tool_name.to_string(),
        ok: !is_error,
        summary,
    });

    Ok((!is_error, parsed))
}
