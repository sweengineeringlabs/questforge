//! Shared OpenAI Chat Completions wire-format adapter used by OpenAI-compatible SPI providers.

use std::path::PathBuf;

use serde_json::{json, Value};

use crate::error::McpError;
use crate::vo::{
    LlmChatMessage, LlmEndpointConfig, LlmHttpRequest, LlmToolInvocation, LlmToolSchema,
    LlmTurnResponse,
};

pub(super) fn home_dir() -> Option<PathBuf> {
    non_empty_env("USERPROFILE")
        .or_else(|| non_empty_env("HOME"))
        .map(PathBuf::from)
}

pub(super) fn read_json_string_path(rel_path: &[&str], json_path: &[&str]) -> Option<String> {
    let mut path = home_dir()?;
    for seg in rel_path {
        path.push(seg);
    }
    let raw = std::fs::read_to_string(path).ok()?;
    let mut cur: Value = serde_json::from_str(&raw).ok()?;
    for key in json_path {
        cur = cur.get(*key)?.clone();
    }
    cur.as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
}

pub(super) fn non_empty_env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.trim().is_empty())
}

pub(super) fn first_env_with_source(vars: &[&str]) -> Option<(String, String)> {
    vars.iter()
        .find_map(|name| non_empty_env(name).map(|val| (val, format!("env:{name}"))))
}

pub(super) fn build_openai_compatible_request(
    config: &LlmEndpointConfig,
    system_prompt: &str,
    messages: &[LlmChatMessage],
    tools: &[LlmToolSchema],
) -> Result<LlmHttpRequest, McpError> {
    let mut headers = vec![("Content-Type".to_string(), "application/json".to_string())];
    if !config.api_key.trim().is_empty() {
        headers.push((
            "Authorization".to_string(),
            format!("Bearer {}", config.api_key),
        ));
    }

    let wire_tools: Vec<Value> = tools
        .iter()
        .map(|t| {
            json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": t.input_schema
                }
            })
        })
        .collect();

    let mut wire_messages = Vec::with_capacity(messages.len() + 1);
    wire_messages.push(json!({
        "role": "system",
        "content": system_prompt
    }));
    for msg in messages {
        match msg {
            LlmChatMessage::User(text) => {
                wire_messages.push(json!({"role": "user", "content": text}));
            }
            LlmChatMessage::Assistant { text, tool_calls } => {
                let mut obj = json!({
                    "role": "assistant",
                    "content": if text.is_empty() { Value::Null } else { Value::String(text.clone()) }
                });
                if !tool_calls.is_empty() {
                    let wire_calls: Vec<Value> = tool_calls
                        .iter()
                        .map(|tc| {
                            let args_str = serde_json::to_string(&tc.arguments)
                                .unwrap_or_else(|_| "{}".to_string());
                            let mut call_obj = json!({
                                "id": tc.id,
                                "type": "function",
                                "function": {
                                    "name": tc.name,
                                    "arguments": args_str
                                }
                            });
                            if let Some(extra) = &tc.extra_content {
                                call_obj["extra_content"] = extra.clone();
                            }
                            call_obj
                        })
                        .collect();
                    obj["tool_calls"] = Value::Array(wire_calls);
                }
                wire_messages.push(obj);
            }
            LlmChatMessage::ToolResult {
                tool_call_id,
                content,
                ..
            } => {
                wire_messages.push(json!({
                    "role": "tool",
                    "tool_call_id": tool_call_id,
                    "content": content
                }));
            }
        }
    }

    Ok(LlmHttpRequest {
        url: config.endpoint_url.clone(),
        headers,
        body: json!({
            "model": config.default_model,
            "messages": wire_messages,
            "tools": wire_tools
        }),
    })
}

pub(super) fn parse_openai_compatible_response(body: &Value) -> Result<LlmTurnResponse, McpError> {
    let message = body
        .get("choices")
        .and_then(|c| c.as_array())
        .and_then(|arr| arr.first())
        .and_then(|choice| choice.get("message"))
        .ok_or_else(|| std::io::Error::other("LLM response missing choices[0].message"))?;

    let text = message
        .get("content")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let raw_calls = message
        .get("tool_calls")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut tool_calls = Vec::with_capacity(raw_calls.len());
    for (idx, tc) in raw_calls.iter().enumerate() {
        let id = tc
            .get("id")
            .and_then(|v| v.as_str())
            .map(ToString::to_string)
            .unwrap_or_else(|| format!("call_{idx}"));
        let func = tc.get("function").unwrap_or(&Value::Null);
        let name = func
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let arguments = match func.get("arguments") {
            Some(Value::String(raw)) => serde_json::from_str(raw).unwrap_or_else(|_| json!({})),
            Some(Value::Object(map)) => Value::Object(map.clone()),
            _ => json!({}),
        };
        let extra_content = tc.get("extra_content").cloned();
        tool_calls.push(LlmToolInvocation {
            id,
            name,
            arguments,
            extra_content,
        });
    }

    Ok(LlmTurnResponse { text, tool_calls })
}
