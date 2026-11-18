//! Integration tests for `questforge-mcp`.

use std::fs;

use questforge_config::EngineKind;
use questforge_init::{InitOptions, ProjectScaffolder};
use questforge_mcp::{
    AgentGoal, AgentOptions, AnthropicProvider, AutoLlmCredentialResolver, GeminiProvider,
    JsonRpcRequest, LlmAgentOptions, LlmChatMessage, LlmEndpointConfig, LlmHttpRequest,
    LlmProviderSpi, LlmToolInvocation, LlmToolSchema, LlmTurnResponse, McpError, OpenAiProvider,
    QuestMcpServer, QuestXrAgent,
};
use serde_json::{json, Value};
use tempfile::TempDir;

#[test]
fn test_mcp_initialize_and_tools_list_happy() {
    let server = QuestMcpServer::default();
    let init_req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "initialize".to_string(),
        params: None,
    };
    let init_resp = server.handle_request(&init_req);
    assert!(matches!(
        init_resp,
        Some(ref r) if r.error.is_none() && r.result.is_some()
    ));

    let list_req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(2)),
        method: "tools/list".to_string(),
        params: None,
    };
    let list_resp = server.handle_request(&list_req);
    assert!(matches!(
        list_resp,
        Some(ref r)
            if r.error.is_none()
                && r.result
                    .as_ref()
                    .and_then(|v| v.get("tools"))
                    .and_then(|t| t.as_array())
                    .is_some_and(|arr| arr.len() == 12)
    ));
}

#[test]
fn test_mcp_tools_call_pipeline_and_analyze_happy() {
    let server = QuestMcpServer::default();
    let gen_req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(3)),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "generate_pipeline",
            "arguments": { "provider": "github" }
        })),
    };
    let gen_resp = server.handle_request(&gen_req);
    assert!(matches!(
        gen_resp,
        Some(ref r)
            if r.result
                .as_ref()
                .and_then(|v| v.get("isError"))
                .and_then(|b| b.as_bool())
                == Some(false)
    ));

    let log_req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(4)),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "analyze_build_failure",
            "arguments": { "log": "IL2CPP error: Android NDK missing" }
        })),
    };
    let log_resp = server.handle_request(&log_req);
    assert!(matches!(
        log_resp,
        Some(ref r)
            if r.result
                .as_ref()
                .and_then(|v| v.get("content"))
                .and_then(|c| c.as_array())
                .is_some_and(|arr| !arr.is_empty())
    ));
}

#[test]
fn test_mcp_unknown_method_and_invalid_tool_error() {
    let Ok(dir) = TempDir::new() else {
        panic!("failed to create tempdir");
    };
    let server = QuestMcpServer::new(dir.path().to_path_buf());
    let bad_method = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(5)),
        method: "unknown/method".to_string(),
        params: None,
    };
    assert!(matches!(
        server.handle_request(&bad_method),
        Some(ref r) if r.error.as_ref().is_some_and(|e| e.code == -32601)
    ));

    let bad_tool = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(6)),
        method: "tools/call".to_string(),
        params: Some(json!({ "name": "nonexistent_tool", "arguments": {} })),
    };
    assert!(matches!(
        server.handle_request(&bad_tool),
        Some(ref r) if r.error.as_ref().is_some_and(|e| e.code == -32602)
    ));
}

#[test]
fn test_xr_agent_in_process_heals_drift_and_completes_release_happy() {
    let Ok(dir) = TempDir::new() else {
        panic!("failed to create tempdir");
    };
    let proj = dir.path().join("agent-quest");
    assert!(ProjectScaffolder::scaffold(&InitOptions {
        target_dir: proj.clone(),
        name: "agent-quest".to_string(),
        package_id: "com.acme.agentquest".to_string(),
        engine: EngineKind::Rust,
        engine_version: None,
        template_url: None,
        force: false,
    })
    .is_ok());

    // Simulate config drift by deleting generated bootstrap so the agent must self-heal.
    assert!(fs::remove_file(proj.join("src/questforge_bootstrap_gen.rs")).is_ok());

    let report = QuestXrAgent::run_in_process(&AgentOptions {
        project_dir: proj.clone(),
        output_dir: proj.join("dist"),
        release_version: "1.0.0".to_string(),
        goal: AgentGoal::Release,
        dry_run: true,
    });
    assert!(matches!(
        report,
        Ok(ref r)
            if r.passed
                && r.discovered_tools.len() == 12
                && r.steps.iter().any(|s| s.tool == "sync_project" && s.ok)
                && r.steps.iter().any(|s| s.tool == "build_project" && s.ok)
    ));
    assert!(proj.join("src/questforge_bootstrap_gen.rs").exists());
}

#[test]
fn test_llm_react_agent_routes_tool_calls_through_mcp_happy() {
    let Ok(dir) = TempDir::new() else {
        panic!("failed to create tempdir");
    };
    let proj = dir.path().join("llm-quest");
    assert!(ProjectScaffolder::scaffold(&InitOptions {
        target_dir: proj.clone(),
        name: "llm-quest".to_string(),
        package_id: "com.acme.llmquest".to_string(),
        engine: EngineKind::Rust,
        engine_version: None,
        template_url: None,
        force: false,
    })
    .is_ok());

    let mut turn = 0_usize;
    let res = QuestXrAgent::run_llm_in_process_with_completer(
        &LlmAgentOptions {
            project_dir: proj.clone(),
            prompt: "Check llm-quest and summarize status".to_string(),
            llm_url: "http://localhost:11434/v1/chat/completions".to_string(),
            model: "test-model".to_string(),
            api_key: String::new(),
            max_turns: 4,
        },
        move |_req_body| {
            turn += 1;
            if turn == 1 {
                Ok(json!({
                    "choices": [{
                        "message": {
                            "role": "assistant",
                            "tool_calls": [{
                                "id": "call_1",
                                "type": "function",
                                "function": {
                                    "name": "check_source",
                                    "arguments": format!("{{\"project_dir\":\"{}\"}}", proj.display().to_string().replace('\\', "\\\\"))
                                }
                            }]
                        }
                    }]
                }))
            } else {
                Ok(json!({
                    "choices": [{
                        "message": {
                            "role": "assistant",
                            "content": "All 5 QuestForge static checks passed via MCP."
                        }
                    }]
                }))
            }
        },
    );

    assert!(matches!(
        res,
        Ok(ref r)
            if r.discovered_tools.len() == 12
                && r.steps.len() == 1
                && r.steps[0].tool == "check_source"
                && r.steps[0].ok
                && r.final_answer.contains("All 5 QuestForge static checks passed")
    ));
}

#[test]
fn test_llm_credential_provider_resolves_configured_providers_happy() {
    assert_eq!(GeminiProvider.provider_id(), "gemini");
    assert_eq!(OpenAiProvider.provider_id(), "openai");
    assert_eq!(AnthropicProvider.provider_id(), "anthropic");
    let _ = AutoLlmCredentialResolver::resolve_from_slice(&[&OpenAiProvider, &AnthropicProvider]);
}

#[test]
fn test_anthropic_spi_wire_format_and_auth_headers_happy() {
    let api_cfg = LlmEndpointConfig {
        provider_id: "anthropic".to_string(),
        endpoint_url: "https://api.anthropic.com/v1/messages".to_string(),
        default_model: "claude-3-5-sonnet-latest".to_string(),
        api_key: "sk-ant-api-test".to_string(),
        auth_source: "env:ANTHROPIC_API_KEY".to_string(),
    };
    let tools = vec![LlmToolSchema {
        name: "check_source".to_string(),
        description: "Run static check".to_string(),
        input_schema: json!({"type": "object"}),
    }];
    let msgs = vec![
        LlmChatMessage::User("Check project".to_string()),
        LlmChatMessage::Assistant {
            text: String::new(),
            tool_calls: vec![LlmToolInvocation {
                id: "toolu_1".to_string(),
                name: "check_source".to_string(),
                arguments: json!({"project_dir": "."}),
                extra_content: None,
            }],
        },
        LlmChatMessage::ToolResult {
            tool_call_id: "toolu_1".to_string(),
            name: "check_source".to_string(),
            content: "{\"ok\":true}".to_string(),
        },
    ];

    let Ok(req) = AnthropicProvider.build_request(&api_cfg, "sys", &msgs, &tools) else {
        panic!("anthropic request");
    };
    assert_eq!(req.url, "https://api.anthropic.com/v1/messages");
    assert!(req
        .headers
        .contains(&("x-api-key".to_string(), "sk-ant-api-test".to_string())));
    assert!(req
        .headers
        .contains(&("anthropic-version".to_string(), "2023-06-01".to_string())));
    assert_eq!(req.body["system"], json!("sys"));
    assert_eq!(req.body["max_tokens"], json!(4096));
    assert_eq!(
        req.body["tools"][0]["input_schema"],
        json!({"type": "object"})
    );
    assert_eq!(
        req.body["messages"][2]["content"][0]["type"],
        json!("tool_result")
    );

    let mut oauth_cfg = api_cfg;
    oauth_cfg.api_key = "oauth-tok".to_string();
    oauth_cfg.auth_source = "env:ANTHROPIC_OAUTH_TOKEN".to_string();
    let Ok(oauth_req) = AnthropicProvider.build_request(
        &oauth_cfg,
        "sys",
        &[LlmChatMessage::User("hi".to_string())],
        &[],
    ) else {
        panic!("anthropic oauth request");
    };
    assert!(oauth_req
        .headers
        .contains(&("Authorization".to_string(), "Bearer oauth-tok".to_string())));
}

#[test]
fn test_llm_agent_anthropic_provider_spi_loop_happy() {
    let Ok(dir) = TempDir::new() else {
        panic!("temp dir");
    };
    let proj = dir.path().join("anthropic-quest");
    assert!(ProjectScaffolder::scaffold(&InitOptions {
        target_dir: proj.clone(),
        name: "anthropic-quest".to_string(),
        package_id: "com.acme.anthropicquest".to_string(),
        engine: EngineKind::Rust,
        engine_version: None,
        template_url: None,
        force: false,
    })
    .is_ok());

    let mut turn = 0usize;
    let res = QuestXrAgent::run_llm_in_process_with_provider(
        &LlmAgentOptions {
            project_dir: proj.clone(),
            prompt: "Check anthropic-quest".to_string(),
            llm_url: "https://api.anthropic.com/v1/messages".to_string(),
            model: "claude-3-5-sonnet-latest".to_string(),
            api_key: "sk-ant-test".to_string(),
            max_turns: 4,
        },
        &AnthropicProvider,
        move |req_body| {
            turn += 1;
            assert!(req_body.get("max_tokens").is_some());
            assert!(req_body.get("system").is_some());
            if turn == 1 {
                Ok(json!({
                    "content": [{
                        "type": "tool_use",
                        "id": "toolu_01",
                        "name": "check_source",
                        "input": { "project_dir": proj }
                    }]
                }))
            } else {
                Ok(json!({
                    "content": [{
                        "type": "text",
                        "text": "Anthropic SPI loop completed static check."
                    }]
                }))
            }
        },
    );

    assert!(matches!(
        res,
        Ok(ref r)
            if r.steps.len() == 1
                && r.steps[0].tool == "check_source"
                && r.steps[0].ok
                && r.final_answer.contains("Anthropic SPI loop completed")
    ));
}

struct CustomSpiProvider;

impl LlmProviderSpi for CustomSpiProvider {
    fn provider_id(&self) -> &'static str {
        "custom-local"
    }

    fn resolve_credentials(&self) -> Option<LlmEndpointConfig> {
        Some(LlmEndpointConfig {
            provider_id: "custom-local".to_string(),
            endpoint_url: "http://localhost:9999/v1/chat".to_string(),
            default_model: "local-v1".to_string(),
            api_key: String::new(),
            auth_source: "local".to_string(),
        })
    }

    fn build_request(
        &self,
        config: &LlmEndpointConfig,
        system_prompt: &str,
        messages: &[LlmChatMessage],
        _tools: &[LlmToolSchema],
    ) -> Result<LlmHttpRequest, McpError> {
        Ok(LlmHttpRequest {
            url: config.endpoint_url.clone(),
            headers: vec![],
            body: json!({
                "model": config.default_model,
                "system": system_prompt,
                "count": messages.len()
            }),
        })
    }

    fn parse_response(&self, body: &Value) -> Result<LlmTurnResponse, McpError> {
        Ok(LlmTurnResponse {
            text: body["reply"].as_str().unwrap_or("").to_string(),
            tool_calls: vec![],
        })
    }
}

#[test]
fn test_custom_llm_provider_spi_implementation_happy() {
    let Ok(dir) = TempDir::new() else {
        panic!("temp dir");
    };
    let custom = CustomSpiProvider;
    assert_eq!(custom.provider_id(), "custom-local");
    let Some(cfg) = custom.resolve_credentials() else {
        panic!("custom cfg");
    };
    let Ok(report) = QuestXrAgent::run_llm_in_process_with_provider(
        &LlmAgentOptions {
            project_dir: dir.path().to_path_buf(),
            prompt: "ping".to_string(),
            llm_url: cfg.endpoint_url,
            model: cfg.default_model,
            api_key: cfg.api_key,
            max_turns: 2,
        },
        &custom,
        |_| Ok(json!({"reply": "pong from custom SPI"})),
    ) else {
        panic!("custom SPI run");
    };
    assert_eq!(report.final_answer, "pong from custom SPI");
}

#[test]
#[ignore = "Requires live network and real provider OAuth/API credentials"]
fn test_live_all_three_providers_real_oauth_e2e() {
    let quest_bin = std::env::current_exe()
        .ok()
        .and_then(|p| {
            p.parent()
                .and_then(|d| d.parent())
                .map(|d| d.join("quest.exe"))
        })
        .filter(|p| p.exists())
        .unwrap_or_else(|| std::path::PathBuf::from("quest"));

    let Ok(dir) = TempDir::new() else {
        panic!("temp dir");
    };
    let proj = dir.path().join("live-quest");
    assert!(ProjectScaffolder::scaffold(&InitOptions {
        target_dir: proj.clone(),
        name: "live-quest".to_string(),
        package_id: "com.acme.livequest".to_string(),
        engine: EngineKind::Rust,
        engine_version: None,
        template_url: None,
        force: false,
    })
    .is_ok());

    let providers: [&dyn LlmProviderSpi; 3] =
        [&GeminiProvider, &OpenAiProvider, &AnthropicProvider];

    let mut failures = Vec::new();
    for provider in providers {
        let id = provider.provider_id();
        let Some(cfg) = provider.resolve_credentials() else {
            failures.push(format!("{id}: resolve_credentials returned None"));
            continue;
        };
        eprintln!(
            "[live-test] provider={id} auth_source={} model={} url={}",
            cfg.auth_source, cfg.default_model, cfg.endpoint_url
        );
        match QuestXrAgent::run_llm_stdio_client_with_spi(
            &quest_bin,
            &proj,
            "Run check_source on this project and summarize in 1 sentence.",
            4,
            provider,
            &cfg,
        ) {
            Ok(report) => {
                eprintln!(
                    "[live-test] provider={id} OK steps={} answer={:?}",
                    report.steps.len(),
                    report.final_answer
                );
            }
            Err(err) => {
                eprintln!("[live-test] provider={id} ERR: {err}");
                failures.push(format!("{id} ({}): {err}", cfg.auth_source));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "Live provider failures: {}",
        failures.join(" | ")
    );
}
