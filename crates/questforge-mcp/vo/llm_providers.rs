//! Value objects for LLM provider contracts and wire translation.

use serde_json::Value;

/// Provider-neutral tool schema passed to [`crate::LlmProviderSpi::build_request`].
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

/// Provider-formatted HTTP request produced by [`crate::LlmProviderSpi::build_request`].
#[derive(Clone, Debug, PartialEq)]
pub struct LlmHttpRequest {
    /// Target endpoint URL.
    pub url: String,
    /// HTTP headers including provider-appropriate authentication.
    pub headers: Vec<(String, String)>,
    /// JSON request payload.
    pub body: Value,
}

/// Provider-neutral turn output parsed by [`crate::LlmProviderSpi::parse_response`].
#[derive(Clone, Debug, PartialEq)]
pub struct LlmTurnResponse {
    /// Assistant text content.
    pub text: String,
    /// Tool calls requested in this turn (empty when the agent has finished).
    pub tool_calls: Vec<LlmToolInvocation>,
}

/// Resolved LLM endpoint and credential bundle returned by [`crate::LlmProviderSpi::resolve_credentials`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LlmEndpointConfig {
    /// Canonical provider identifier (`gemini`, `openai`, `anthropic`).
    pub provider_id: String,
    /// Endpoint URL for the provider's chat/messages API.
    pub endpoint_url: String,
    /// Recommended default model identifier.
    pub default_model: String,
    /// Resolved OAuth token or API key.
    pub api_key: String,
    /// Human-readable credential source (`gcloud-oauth`, `env:OPENAI_API_KEY`, etc.).
    pub auth_source: String,
}
