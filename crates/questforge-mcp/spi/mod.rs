//! Service Provider Interface (SPI) implementations for LLM providers.

mod anthropic_provider;
mod gemini_provider;
mod openai_provider;
mod openai_wire;

pub use self::anthropic_provider::AnthropicProvider;
pub use self::gemini_provider::GeminiProvider;
pub use self::openai_provider::OpenAiProvider;

use std::process::Command;

use crate::error::McpError;
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

    /// Runs the provider's interactive OAuth login hook when credentials are missing or `force` is true.
    pub fn authenticate_provider(
        provider: &dyn LlmProviderSpi,
        force: bool,
    ) -> Result<LlmEndpointConfig, McpError> {
        if !force {
            if let Some(cfg) = provider.resolve_credentials() {
                return Ok(cfg);
            }
        }

        let (bin, args): (&str, &[&str]) = match provider.provider_id() {
            "gemini" => (
                if cfg!(windows) {
                    "gcloud.cmd"
                } else {
                    "gcloud"
                },
                &["auth", "application-default", "login"],
            ),
            "openai" => (
                if cfg!(windows) { "codex.cmd" } else { "codex" },
                &["login"],
            ),
            "anthropic" => (
                if cfg!(windows) {
                    "claude.cmd"
                } else {
                    "claude"
                },
                &["auth", "login"],
            ),
            other => {
                return Err(std::io::Error::other(format!(
                    "no OAuth login hook registered for provider '{other}'"
                ))
                .into());
            }
        };

        let status = Command::new(bin).args(args).status().map_err(|err| {
            std::io::Error::other(format!(
                "failed to launch '{bin} {}': {err}",
                args.join(" ")
            ))
        })?;
        if !status.success() {
            return Err(std::io::Error::other(format!(
                "OAuth login command '{bin} {}' exited with {status}",
                args.join(" ")
            ))
            .into());
        }

        provider.resolve_credentials().ok_or_else(|| {
            std::io::Error::other(format!(
                "OAuth login completed for '{}', but no credentials were discovered afterward",
                provider.provider_id()
            ))
            .into()
        })
    }

    /// Canonical API-key environment variable name for a built-in provider.
    #[must_use]
    pub fn provider_api_key_env(provider_id: &str) -> &'static str {
        match provider_id {
            "gemini" => "GEMINI_API_KEY",
            "anthropic" => "ANTHROPIC_API_KEY",
            _ => "OPENAI_API_KEY",
        }
    }

    /// Builds an [`LlmEndpointConfig`] from a caller-supplied or prompted API key.
    #[must_use]
    pub fn config_with_api_key(provider_id: &str, api_key: String) -> LlmEndpointConfig {
        match provider_id {
            "gemini" => LlmEndpointConfig {
                provider_id: "gemini".to_string(),
                endpoint_url:
                    "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions"
                        .to_string(),
                default_model: "gemini-flash-lite-latest".to_string(),
                api_key,
                auth_source: "prompt:api-key".to_string(),
            },
            "anthropic" => LlmEndpointConfig {
                provider_id: "anthropic".to_string(),
                endpoint_url: "https://api.anthropic.com/v1/messages".to_string(),
                default_model: "claude-3-5-sonnet-latest".to_string(),
                api_key,
                auth_source: "prompt:api-key".to_string(),
            },
            _ => LlmEndpointConfig {
                provider_id: "openai".to_string(),
                endpoint_url: "https://api.openai.com/v1/chat/completions".to_string(),
                default_model: "gpt-4o-mini".to_string(),
                api_key,
                auth_source: "prompt:api-key".to_string(),
            },
        }
    }

    /// Returns `true` when an [`McpError`] represents an HTTP 401/403 authentication rejection.
    #[must_use]
    pub fn is_auth_error(err: &McpError) -> bool {
        let msg = err.to_string();
        msg.contains("LLM HTTP 401")
            || msg.contains("LLM HTTP 403")
            || msg.contains("token_expired")
            || msg.contains("oauth_not_allowed")
    }
}
