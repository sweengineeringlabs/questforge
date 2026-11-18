//! Integration tests for `questforge-mcp`.

use std::fs;

use questforge_config::EngineKind;
use questforge_init::{InitOptions, ProjectScaffolder};
use questforge_mcp::{
    AgentGoal, AgentOptions, AutoLlmCredentialResolver, JsonRpcRequest, LlmAgentOptions,
    LlmCredentialProvider, QuestMcpServer, QuestXrAgent,
};
use serde_json::json;
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
    assert_eq!(LlmCredentialProvider::Gemini.provider_id(), "gemini");
    assert_eq!(LlmCredentialProvider::OpenAi.provider_id(), "openai");
    assert_eq!(LlmCredentialProvider::Anthropic.provider_id(), "anthropic");
    let _ = AutoLlmCredentialResolver::resolve_from_slice(&[
        LlmCredentialProvider::OpenAi,
        LlmCredentialProvider::Anthropic,
    ]);
}
