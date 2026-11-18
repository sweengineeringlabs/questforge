//! OpenAI [`LlmProviderSpi`] implementation (`OPENAI_OAUTH_TOKEN` -> `OPENAI_API_KEY`).

use serde_json::Value;

use crate::error::McpError;
use crate::spi::openai_wire::{
    build_openai_compatible_request, first_env_with_source, parse_openai_compatible_response,
};
use crate::traits::LlmProviderSpi;
use crate::vo::{
    LlmChatMessage, LlmEndpointConfig, LlmHttpRequest, LlmToolSchema, LlmTurnResponse,
};

/// OpenAI provider using `/v1/chat/completions`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OpenAiProvider;

impl LlmProviderSpi for OpenAiProvider {
    fn provider_id(&self) -> &'static str {
        "openai"
    }

    fn resolve_credentials(&self) -> Option<LlmEndpointConfig> {
        let (key, source) = first_env_with_source(&["OPENAI_OAUTH_TOKEN", "OPENAI_API_KEY"])?;

        Some(LlmEndpointConfig {
            provider_id: self.provider_id().to_string(),
            endpoint_url: "https://api.openai.com/v1/chat/completions".to_string(),
            default_model: "gpt-4o-mini".to_string(),
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
        build_openai_compatible_request(config, system_prompt, messages, tools)
    }

    fn parse_response(&self, body: &Value) -> Result<LlmTurnResponse, McpError> {
        parse_openai_compatible_response(body)
    }
}
