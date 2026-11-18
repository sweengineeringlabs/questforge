//! Anthropic [`LlmProviderSpi`] implementation (`ANTHROPIC_OAUTH_TOKEN` -> `ANTHROPIC_API_KEY`).

use serde_json::{json, Value};

use crate::error::McpError;
use crate::spi::openai_wire::first_env_with_source;
use crate::traits::LlmProviderSpi;
use crate::vo::{
    LlmChatMessage, LlmEndpointConfig, LlmHttpRequest, LlmToolInvocation, LlmToolSchema,
    LlmTurnResponse,
};

/// Anthropic Messages API provider (`/v1/messages`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AnthropicProvider;

impl LlmProviderSpi for AnthropicProvider {
    fn provider_id(&self) -> &'static str {
        "anthropic"
    }

    fn resolve_credentials(&self) -> Option<LlmEndpointConfig> {
        let (key, source) = first_env_with_source(&["ANTHROPIC_OAUTH_TOKEN", "ANTHROPIC_API_KEY"])?;

        Some(LlmEndpointConfig {
            provider_id: self.provider_id().to_string(),
            endpoint_url: "https://api.anthropic.com/v1/messages".to_string(),
            default_model: "claude-3-5-sonnet-latest".to_string(),
            api_key: key,
            auth_source: source,
        })
    }

    fn build_request(
        &self,
        config: &LlmEndpointConfig,
        system_prompt: &str,
        messages: &[LlmChatMessage],
        tools: &[LlmToolSchema],
    ) -> Result<LlmHttpRequest, McpError> {
        let mut headers = vec![
            ("Content-Type".to_string(), "application/json".to_string()),
            ("anthropic-version".to_string(), "2023-06-01".to_string()),
        ];
        if !config.api_key.trim().is_empty() {
            if config.auth_source.to_ascii_lowercase().contains("oauth") {
                headers.push((
                    "Authorization".to_string(),
                    format!("Bearer {}", config.api_key),
                ));
            } else {
                headers.push(("x-api-key".to_string(), config.api_key.clone()));
            }
        }

        let wire_tools: Vec<Value> = tools
            .iter()
            .map(|t| {
                json!({
                    "name": t.name,
                    "description": t.description,
                    "input_schema": t.input_schema
                })
            })
            .collect();

        let mut wire_messages: Vec<Value> = Vec::with_capacity(messages.len());
        let mut pending_tool_results: Vec<Value> = Vec::new();
        let flush_tool_results = |wire: &mut Vec<Value>, pending: &mut Vec<Value>| {
            if !pending.is_empty() {
                wire.push(json!({
                    "role": "user",
                    "content": std::mem::take(pending)
                }));
            }
        };

        for msg in messages {
            match msg {
                LlmChatMessage::User(text) => {
                    flush_tool_results(&mut wire_messages, &mut pending_tool_results);
                    wire_messages.push(json!({
                        "role": "user",
                        "content": text
                    }));
                }
                LlmChatMessage::Assistant { text, tool_calls } => {
                    flush_tool_results(&mut wire_messages, &mut pending_tool_results);
                    let mut blocks = Vec::new();
                    if !text.is_empty() {
                        blocks.push(json!({
                            "type": "text",
                            "text": text
                        }));
                    }
                    for tc in tool_calls {
                        blocks.push(json!({
                            "type": "tool_use",
                            "id": tc.id,
                            "name": tc.name,
                            "input": tc.arguments
                        }));
                    }
                    wire_messages.push(json!({
                        "role": "assistant",
                        "content": blocks
                    }));
                }
                LlmChatMessage::ToolResult {
                    tool_call_id,
                    content,
                    ..
                } => {
                    pending_tool_results.push(json!({
                        "type": "tool_result",
                        "tool_use_id": tool_call_id,
                        "content": content
                    }));
                }
            }
        }
        flush_tool_results(&mut wire_messages, &mut pending_tool_results);

        Ok(LlmHttpRequest {
            url: config.endpoint_url.clone(),
            headers,
            body: json!({
                "model": config.default_model,
                "max_tokens": 4096,
                "system": system_prompt,
                "messages": wire_messages,
                "tools": wire_tools
            }),
        })
    }

    fn parse_response(&self, body: &Value) -> Result<LlmTurnResponse, McpError> {
        let blocks = body
            .get("content")
            .and_then(|v| v.as_array())
            .ok_or_else(|| std::io::Error::other("Anthropic response missing content array"))?;

        let mut text_parts = Vec::new();
        let mut tool_calls = Vec::new();
        for (idx, block) in blocks.iter().enumerate() {
            match block.get("type").and_then(|v| v.as_str()) {
                Some("text") => {
                    if let Some(t) = block.get("text").and_then(|v| v.as_str()) {
                        text_parts.push(t);
                    }
                }
                Some("tool_use") => {
                    let id = block
                        .get("id")
                        .and_then(|v| v.as_str())
                        .map(ToString::to_string)
                        .unwrap_or_else(|| format!("toolu_{idx}"));
                    let name = block
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let arguments = block.get("input").cloned().unwrap_or_else(|| json!({}));
                    tool_calls.push(LlmToolInvocation {
                        id,
                        name,
                        arguments,
                    });
                }
                _ => {}
            }
        }

        Ok(LlmTurnResponse {
            text: text_parts.join("\n"),
            tool_calls,
        })
    }
}
