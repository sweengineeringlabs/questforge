//! Concrete [`LlmCredentialProvider`] SPI implementations (OAuth first, API-key fallback).

use std::process::Command;

use questforge_api::{LlmCredentialProvider, LlmEndpointConfig};

/// Gemini credential provider (`gcloud` OAuth -> `GEMINI_API_KEY` / `GOOGLE_API_KEY`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GeminiCredentialProvider;

impl LlmCredentialProvider for GeminiCredentialProvider {
    fn provider_id(&self) -> &'static str {
        "gemini"
    }

    fn resolve(&self) -> Option<LlmEndpointConfig> {
        let (key, source) = if let Some(k) = discover_gemini_key_via_gcloud() {
            (k, "gcloud-oauth".to_string())
        } else if let Some(k) = non_empty_env("GEMINI_API_KEY") {
            (k, "env:GEMINI_API_KEY".to_string())
        } else if let Some(k) = non_empty_env("GOOGLE_API_KEY") {
            (k, "env:GOOGLE_API_KEY".to_string())
        } else {
            return None;
        };

        Some(LlmEndpointConfig {
            provider_id: self.provider_id().to_string(),
            endpoint_url:
                "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions"
                    .to_string(),
            default_model: "gemini-flash-lite-latest".to_string(),
            api_key: key,
            auth_source: source,
        })
    }
}

/// OpenAI credential provider (`OPENAI_OAUTH_TOKEN` -> `OPENAI_API_KEY`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OpenAiCredentialProvider;

impl LlmCredentialProvider for OpenAiCredentialProvider {
    fn provider_id(&self) -> &'static str {
        "openai"
    }

    fn resolve(&self) -> Option<LlmEndpointConfig> {
        let (key, source) = if let Some(k) = non_empty_env("OPENAI_OAUTH_TOKEN") {
            (k, "env:OPENAI_OAUTH_TOKEN".to_string())
        } else if let Some(k) = non_empty_env("OPENAI_API_KEY") {
            (k, "env:OPENAI_API_KEY".to_string())
        } else {
            return None;
        };

        Some(LlmEndpointConfig {
            provider_id: self.provider_id().to_string(),
            endpoint_url: "https://api.openai.com/v1/chat/completions".to_string(),
            default_model: "gpt-4o-mini".to_string(),
            api_key: key,
            auth_source: source,
        })
    }
}

/// Anthropic credential provider (`ANTHROPIC_OAUTH_TOKEN` -> `ANTHROPIC_API_KEY`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AnthropicCredentialProvider;

impl LlmCredentialProvider for AnthropicCredentialProvider {
    fn provider_id(&self) -> &'static str {
        "anthropic"
    }

    fn resolve(&self) -> Option<LlmEndpointConfig> {
        let (key, source) = if let Some(k) = non_empty_env("ANTHROPIC_OAUTH_TOKEN") {
            (k, "env:ANTHROPIC_OAUTH_TOKEN".to_string())
        } else if let Some(k) = non_empty_env("ANTHROPIC_API_KEY") {
            (k, "env:ANTHROPIC_API_KEY".to_string())
        } else {
            return None;
        };

        Some(LlmEndpointConfig {
            provider_id: self.provider_id().to_string(),
            endpoint_url: "https://api.anthropic.com/v1/chat/completions".to_string(),
            default_model: "claude-3-5-sonnet-latest".to_string(),
            api_key: key,
            auth_source: source,
        })
    }
}

/// Resolves the first available LLM provider via the [`LlmCredentialProvider`] SPI.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AutoLlmCredentialResolver;

impl AutoLlmCredentialResolver {
    /// Iterates registered [`LlmCredentialProvider`] SPIs in priority order (`gemini` -> `openai` -> `anthropic`).
    #[must_use]
    pub fn resolve_first() -> Option<LlmEndpointConfig> {
        let gemini = GeminiCredentialProvider;
        let openai = OpenAiCredentialProvider;
        let anthropic = AnthropicCredentialProvider;
        let providers: [&dyn LlmCredentialProvider; 3] = [&gemini, &openai, &anthropic];
        Self::resolve_from_slice(&providers)
    }

    /// Resolves the first matching config from a caller-supplied slice of [`LlmCredentialProvider`] SPIs.
    #[must_use]
    pub fn resolve_from_slice(
        providers: &[&dyn LlmCredentialProvider],
    ) -> Option<LlmEndpointConfig> {
        providers.iter().find_map(|p| p.resolve())
    }
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
