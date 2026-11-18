//! Google Gemini [`LlmProviderSpi`] implementation (gcloud OAuth -> `GEMINI_API_KEY` / `GOOGLE_API_KEY`).

use std::process::Command;

use serde_json::Value;

use crate::error::McpError;
use crate::spi::openai_wire::{
    build_openai_compatible_request, first_env_with_source, parse_openai_compatible_response,
};
use crate::traits::LlmProviderSpi;
use crate::vo::{
    LlmChatMessage, LlmEndpointConfig, LlmHttpRequest, LlmToolSchema, LlmTurnResponse,
};

/// Google Gemini provider using OpenAI-compatible Chat Completions endpoint.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GeminiProvider;

impl LlmProviderSpi for GeminiProvider {
    fn provider_id(&self) -> &'static str {
        "gemini"
    }

    fn resolve_credentials(&self) -> Option<LlmEndpointConfig> {
        let (key, source) = discover_gemini_key_via_gcloud()
            .map(|k| (k, "gcloud-oauth".to_string()))
            .or_else(|| first_env_with_source(&["GEMINI_API_KEY", "GOOGLE_API_KEY"]))?;

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
