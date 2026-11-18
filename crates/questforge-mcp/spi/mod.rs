//! Service Provider Interface (SPI) implementations for LLM providers.

mod anthropic_provider;
mod gemini_provider;
mod openai_provider;
mod openai_wire;

pub use self::anthropic_provider::AnthropicProvider;
pub use self::gemini_provider::GeminiProvider;
pub use self::openai_provider::OpenAiProvider;

use crate::traits::LlmProviderSpi;
use crate::vo::LlmEndpointConfig;

/// Resolves the first available built-in [`LlmProviderSpi`] (`gemini` -> `openai` -> `anthropic`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AutoLlmCredentialResolver;

impl AutoLlmCredentialResolver {
    /// Built-in [`LlmProviderSpi`] implementations in priority order.
    #[must_use]
    pub fn built_in_providers() -> [&'static dyn LlmProviderSpi; 3] {
        [&GeminiProvider, &OpenAiProvider, &AnthropicProvider]
    }

    /// Looks up a built-in [`LlmProviderSpi`] by canonical identifier (`gemini`, `openai`, `anthropic`).
    #[must_use]
    pub fn provider_for_id(id: &str) -> Option<&'static dyn LlmProviderSpi> {
        let lower = id.trim().to_ascii_lowercase();
        Self::built_in_providers()
            .into_iter()
            .find(|p| p.provider_id() == lower)
    }

    /// Infers the matching built-in [`LlmProviderSpi`] from an endpoint URL.
    #[must_use]
    pub fn provider_for_url(url: &str) -> &'static dyn LlmProviderSpi {
        let lower = url.to_ascii_lowercase();
        if lower.contains("anthropic.com") {
            &AnthropicProvider
        } else if lower.contains("generativelanguage.googleapis.com") {
            &GeminiProvider
        } else {
            &OpenAiProvider
        }
    }

    /// Iterates built-in [`LlmProviderSpi`]s and returns the first resolved [`LlmEndpointConfig`].
    #[must_use]
    pub fn resolve_first() -> Option<LlmEndpointConfig> {
        Self::resolve_first_with_provider().map(|(_, cfg)| cfg)
    }

    /// Iterates built-in [`LlmProviderSpi`]s and returns both the provider SPI and resolved config.
    #[must_use]
    pub fn resolve_first_with_provider() -> Option<(&'static dyn LlmProviderSpi, LlmEndpointConfig)>
    {
        Self::built_in_providers()
            .into_iter()
            .find_map(|p| p.resolve_credentials().map(|cfg| (p, cfg)))
    }

    /// Resolves the first matching config from a caller-supplied slice of [`LlmProviderSpi`]s.
    #[must_use]
    pub fn resolve_from_slice<'a>(
        providers: &[&'a dyn LlmProviderSpi],
    ) -> Option<(&'a dyn LlmProviderSpi, LlmEndpointConfig)> {
        providers
            .iter()
            .find_map(|p| p.resolve_credentials().map(|cfg| (*p, cfg)))
    }
}
