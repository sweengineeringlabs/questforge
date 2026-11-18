//! LLM credential, endpoint, and wire-format providers (OAuth first, API-key fallback).

use std::process::Command;

use serde_json::{json, Value};

use crate::error::McpError;
use crate::traits::LlmProviderSpi;

/// Provider-neutral tool schema passed to [`LlmProviderSpi::build_request`].
#[derive(Clone, Debug, PartialEq)]
pub struct LlmToolSchema {
    /// Tool identifier.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// JSON Schema for tool arguments.
    pub input_schema: Value,
}

/// Provider-neutral tool invocation requested by an LLM turn.
#[derive(Clone, Debug, PartialEq)]
pub struct LlmToolInvocation {
    /// Provider-assigned tool call identifier.
    pub id: String,
    /// Tool name to invoke.
    pub name: String,
    /// Parsed JSON argument object.
    pub arguments: Value,
}

/// Provider-neutral conversation message used across LLM turns.
#[derive(Clone, Debug, PartialEq)]
pub enum LlmChatMessage {
    /// User prompt message.
    User(String),
    /// Assistant response containing optional text and tool calls.
    Assistant {
        /// Assistant text content (may be empty when tool calls are emitted).
        text: String,
        /// Tool calls emitted by the assistant.
        tool_calls: Vec<LlmToolInvocation>,
    },
    /// Tool execution output returned to the model.
    ToolResult {
        /// Matching [`LlmToolInvocation::id`].
        tool_call_id: String,
        /// Tool name.
        name: String,
        /// Serialized tool output.
        content: String,
    },
}

/// Provider-formatted HTTP request produced by [`LlmProviderSpi::build_request`].
#[derive(Clone, Debug, PartialEq)]
pub struct LlmHttpRequest {
    /// Target endpoint URL.
    pub url: String,
    /// HTTP headers including provider-appropriate authentication.
    pub headers: Vec<(String, String)>,
    /// JSON request payload.
    pub body: Value,
}

/// Provider-neutral turn output parsed by [`LlmProviderSpi::parse_response`].
#[derive(Clone, Debug, PartialEq)]
pub struct LlmTurnResponse {
    /// Assistant text content.
    pub text: String,
    /// Tool calls requested in this turn (empty when the agent has finished).
    pub tool_calls: Vec<LlmToolInvocation>,
}

/// Resolved LLM endpoint and credential bundle returned by [`LlmCredentialProvider`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LlmEndpointConfig {
    /// Canonical provider identifier (`gemini`, `openai`, `anthropic`).
    pub provider_id: String,
    /// Chat completions endpoint URL.
    pub endpoint_url: String,
    /// Recommended default model identifier.
    pub default_model: String,
    /// Resolved OAuth token or API key.
    pub api_key: String,
    /// Human-readable credential source (`gcloud-oauth`, `env:OPENAI_API_KEY`, etc.).
    pub auth_source: String,
}

/// Supported LLM credential providers (OAuth first, API-key fallback).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LlmCredentialProvider {
    /// Google Gemini (`gcloud` OAuth -> `GEMINI_API_KEY` / `GOOGLE_API_KEY`).
    Gemini,
    /// OpenAI (`OPENAI_OAUTH_TOKEN` -> `OPENAI_API_KEY`).
    OpenAi,
    /// Anthropic (`ANTHROPIC_OAUTH_TOKEN` -> `ANTHROPIC_API_KEY`).
    Anthropic,
}

impl LlmCredentialProvider {
    /// Canonical provider identifier (`gemini`, `openai`, `anthropic`).
    #[must_use]
    pub const fn provider_id(self) -> &'static str {
        match self {
            Self::Gemini => "gemini",
            Self::OpenAi => "openai",
            Self::Anthropic => "anthropic",
        }
    }

    /// Infers a built-in provider from an endpoint URL.
    #[must_use]
    pub fn from_url(url: &str) -> Self {
        let lower = url.to_ascii_lowercase();
        if lower.contains("anthropic.com") {
            Self::Anthropic
        } else if lower.contains("generativelanguage.googleapis.com") {
            Self::Gemini
        } else {
            Self::OpenAi
        }
    }

    /// Attempts to resolve credentials (OAuth first, API-key fallback) for this provider.
    #[must_use]
    pub fn resolve(self) -> Option<LlmEndpointConfig> {
        match self {
            Self::Gemini => resolve_gemini(),
            Self::OpenAi => resolve_openai(),
            Self::Anthropic => resolve_anthropic(),
        }
    }
}

impl LlmProviderSpi for LlmCredentialProvider {
    fn provider_id(&self) -> &'static str {
        (*self).provider_id()
    }

    fn resolve_credentials(&self) -> Option<LlmEndpointConfig> {
        self.resolve()
    }

    fn build_request(
        &self,
        config: &LlmEndpointConfig,
        system_prompt: &str,
        messages: &[LlmChatMessage],
        tools: &[LlmToolSchema],
    ) -> Result<LlmHttpRequest, McpError> {
        match self {
            Self::Gemini | Self::OpenAi => {
                build_openai_request(config, system_prompt, messages, tools)
            }
            Self::Anthropic => build_anthropic_request(config, system_prompt, messages, tools),
        }
    }

    fn parse_response(&self, body: &Value) -> Result<LlmTurnResponse, McpError> {
        match self {
            Self::Gemini | Self::OpenAi => parse_openai_response(body),
            Self::Anthropic => parse_anthropic_response(body),
        }
    }
}

/// Resolves the first available LLM provider (`gemini` -> `openai` -> `anthropic`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AutoLlmCredentialResolver;

impl AutoLlmCredentialResolver {
    /// Iterates built-in [`LlmCredentialProvider`]s in priority order (`gemini` -> `openai` -> `anthropic`).
    #[must_use]
    pub fn resolve_first() -> Option<LlmEndpointConfig> {
        Self::resolve_first_with_provider().map(|(_, cfg)| cfg)
    }

    /// Iterates built-in [`LlmCredentialProvider`]s and returns both the provider SPI and resolved config.
    #[must_use]
    pub fn resolve_first_with_provider() -> Option<(LlmCredentialProvider, LlmEndpointConfig)> {
        [
            LlmCredentialProvider::Gemini,
            LlmCredentialProvider::OpenAi,
            LlmCredentialProvider::Anthropic,
        ]
        .into_iter()
        .find_map(|p| p.resolve().map(|cfg| (p, cfg)))
    }

    /// Resolves the first matching config from a caller-supplied slice of [`LlmCredentialProvider`]s.
    #[must_use]
    pub fn resolve_from_slice(providers: &[LlmCredentialProvider]) -> Option<LlmEndpointConfig> {
        providers.iter().find_map(|p| p.resolve())
    }
}

fn first_env_with_source(vars: &[&str]) -> Option<(String, String)> {
    vars.iter()
        .find_map(|name| non_empty_env(name).map(|val| (val, format!("env:{name}"))))
}

fn resolve_gemini() -> Option<LlmEndpointConfig> {
    let (key, source) = discover_gemini_key_via_gcloud()
        .map(|k| (k, "gcloud-oauth".to_string()))
        .or_else(|| first_env_with_source(&["GEMINI_API_KEY", "GOOGLE_API_KEY"]))?;

    Some(LlmEndpointConfig {
        provider_id: "gemini".to_string(),
        endpoint_url: "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions"
            .to_string(),
        default_model: "gemini-flash-lite-latest".to_string(),
        api_key: key,
        auth_source: source,
    })
}

fn resolve_openai() -> Option<LlmEndpointConfig> {
    let (key, source) = first_env_with_source(&["OPENAI_OAUTH_TOKEN", "OPENAI_API_KEY"])?;

    Some(LlmEndpointConfig {
        provider_id: "openai".to_string(),
        endpoint_url: "https://api.openai.com/v1/chat/completions".to_string(),
        default_model: "gpt-4o-mini".to_string(),
        api_key: key,
        auth_source: source,
    })
}

fn resolve_anthropic() -> Option<LlmEndpointConfig> {
    let (key, source) = first_env_with_source(&["ANTHROPIC_OAUTH_TOKEN", "ANTHROPIC_API_KEY"])?;

    Some(LlmEndpointConfig {
        provider_id: "anthropic".to_string(),
        endpoint_url: "https://api.anthropic.com/v1/messages".to_string(),
        default_model: "claude-3-5-sonnet-latest".to_string(),
        api_key: key,
        auth_source: source,
    })
}

fn build_openai_request(
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
                            json!({
                                "id": tc.id,
                                "type": "function",
                                "function": {
                                    "name": tc.name,
                                    "arguments": args_str
                                }
                            })
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

fn parse_openai_response(body: &Value) -> Result<LlmTurnResponse, McpError> {
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
        tool_calls.push(LlmToolInvocation {
            id,
            name,
            arguments,
        });
    }

    Ok(LlmTurnResponse { text, tool_calls })
}

fn build_anthropic_request(
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

fn parse_anthropic_response(body: &Value) -> Result<LlmTurnResponse, McpError> {
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

fn non_empty_env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.trim().is_empty())
}

fn discover_gemini_key_via_gcloud() -> Option<String> {
    let gcloud = if cfg!(windows) {
        "gcloud.cmd"
    } else {
        "gcloud"
    };
    let projects_out = Command::new(gcloud)
        .args([
            "projects",
            "list",
            "--filter=projectId:gen-lang-client*",
            "--format=value(projectId)",
            "--limit=5",
        ])
        .output()
        .ok()?;
    if !projects_out.status.success() {
        return None;
    }
    for proj in String::from_utf8_lossy(&projects_out.stdout).lines() {
        let proj = proj.trim();
        if proj.is_empty() {
            continue;
        }
        let keys_out = Command::new(gcloud)
            .args([
                "services",
                "api-keys",
                "list",
                "--project",
                proj,
                "--filter=restrictions.apiTargets.service:generativelanguage.googleapis.com",
                "--format=value(uid)",
                "--limit=1",
            ])
            .output()
            .ok()?;
        let uid = String::from_utf8_lossy(&keys_out.stdout).trim().to_string();
        if uid.is_empty() {
            continue;
        }
        let str_out = Command::new(gcloud)
            .args([
                "services",
                "api-keys",
                "get-key-string",
                &uid,
                "--project",
                proj,
                "--format=value(keyString)",
            ])
            .output()
            .ok()?;
        if str_out.status.success() {
            let key = String::from_utf8_lossy(&str_out.stdout).trim().to_string();
            if !key.is_empty() {
                return Some(key);
            }
        }
    }
    None
}
