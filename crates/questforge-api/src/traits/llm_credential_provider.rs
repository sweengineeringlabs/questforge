//! Service Provider Interface (SPI) for LLM credential and endpoint resolution.

/// Resolved LLM endpoint and credential bundle returned by an [`LlmCredentialProvider`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LlmEndpointConfig {
    /// Canonical provider identifier (`gemini`, `openai`, `anthropic`, `ollama`).
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

/// SPI contract implemented by LLM credential/OAuth providers.
pub trait LlmCredentialProvider {
    /// Canonical provider identifier (`gemini`, `openai`, `anthropic`, `ollama`).
    fn provider_id(&self) -> &'static str;

    /// Attempts to resolve credentials (OAuth first, API-key fallback) for this provider.
    fn resolve(&self) -> Option<LlmEndpointConfig>;
}
