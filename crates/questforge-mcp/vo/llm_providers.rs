//! LLM credential and endpoint providers (OAuth first, API-key fallback).

use std::process::Command;

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

/// Resolves the first available LLM provider (`gemini` -> `openai` -> `anthropic`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AutoLlmCredentialResolver;

impl AutoLlmCredentialResolver {
    /// Iterates built-in [`LlmCredentialProvider`]s in priority order (`gemini` -> `openai` -> `anthropic`).
    #[must_use]
    pub fn resolve_first() -> Option<LlmEndpointConfig> {
        Self::resolve_from_slice(&[
            LlmCredentialProvider::Gemini,
            LlmCredentialProvider::OpenAi,
            LlmCredentialProvider::Anthropic,
        ])
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
        endpoint_url: "https://api.anthropic.com/v1/chat/completions".to_string(),
        default_model: "claude-3-5-sonnet-latest".to_string(),
        api_key: key,
        auth_source: source,
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
