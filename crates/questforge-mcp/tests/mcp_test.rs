//! Integration tests for `questforge-mcp`.

use questforge_mcp::{JsonRpcRequest, QuestMcpServer};
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
                    .is_some_and(|arr| arr.len() == 11)
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
