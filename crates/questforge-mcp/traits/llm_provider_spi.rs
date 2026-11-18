//! LLM provider Service Provider Interface (SPI) trait.

use serde_json::Value;

use crate::error::McpError;
use crate::vo::{
    LlmChatMessage, LlmEndpointConfig, LlmHttpRequest, LlmToolSchema, LlmTurnResponse,
};

/// Service Provider Interface for LLM credential resolution and wire-format translation.
pub trait LlmProviderSpi: Send + Sync {
    /// Canonical provider identifier (e.g. `"gemini"`, `"openai"`, `"anthropic"`).
    fn provider_id(&self) -> &'static str;

    /// Resolves OAuth or API-key credentials and default endpoint metadata for this provider.
    fn resolve_credentials(&self) -> Option<LlmEndpointConfig>;

    /// Builds an HTTP request (URL, auth headers, and provider-specific JSON body).
    fn build_request(
        &self,
        config: &LlmEndpointConfig,
        system_prompt: &str,
        messages: &[LlmChatMessage],
        tools: &[LlmToolSchema],
    ) -> Result<LlmHttpRequest, McpError>;

    /// Parses a provider-specific JSON response into a provider-neutral [`LlmTurnResponse`].
    fn parse_response(&self, body: &Value) -> Result<LlmTurnResponse, McpError>;
}
