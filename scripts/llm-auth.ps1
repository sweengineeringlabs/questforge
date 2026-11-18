<#
.SYNOPSIS
Authenticate or resolve credentials (OAuth first, API-key fallback) for QuestForge LLM providers
(gemini, openai, anthropic, ollama) and configure QUESTFORGE_LLM_* environment variables.

.EXAMPLE
. .\scripts\llm-auth.ps1 -Provider gemini
. .\scripts\llm-auth.ps1 -Provider openai
#>
[CmdletBinding()]
param(
    [ValidateSet("auto", "gemini", "openai", "anthropic", "ollama")]
    [string]$Provider = "auto",
    [switch]$ForceLogin
)

$ErrorActionPreference = "Stop"

function Resolve-Gemini {
    if ($env:GEMINI_API_KEY) {
        return @{ url = "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions"; model = "gemini-flash-latest"; key = $env:GEMINI_API_KEY; source = "GEMINI_API_KEY" }
    }
    if (Get-Command gcloud -ErrorAction SilentlyContinue) {
        if ($ForceLogin) {
            & gcloud auth application-default login
        }
        $proj = (& gcloud projects list --filter="projectId:gen-lang-client*" --format="value(projectId)" --limit=1 2>$null | Select-Object -First 1)
        if ($proj) {
            $uid = (& gcloud services api-keys list --project $proj --filter="restrictions.apiTargets.service:generativelanguage.googleapis.com" --format="value(uid)" --limit=1 2>$null | Select-Object -First 1)
            if ($uid) {
                $key = (& gcloud services api-keys get-key-string $uid --project $proj --format="value(keyString)" 2>$null).Trim()
                if ($key) {
                    return @{ url = "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions"; model = "gemini-flash-latest"; key = $key; source = "gcloud-oauth ($proj)" }
                }
            }
        }
    }
    if ($env:GOOGLE_API_KEY) {
        return @{ url = "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions"; model = "gemini-flash-latest"; key = $env:GOOGLE_API_KEY; source = "GOOGLE_API_KEY" }
    }
    return $null
}

function Resolve-OpenAI {
    if ($env:OPENAI_OAUTH_TOKEN) {
        return @{ url = "https://api.openai.com/v1/chat/completions"; model = "gpt-4o-mini"; key = $env:OPENAI_OAUTH_TOKEN; source = "OPENAI_OAUTH_TOKEN" }
    }
    $codexAuth = Join-Path $env:USERPROFILE ".codex\auth.json"
    if (Test-Path $codexAuth) {
        try {
            $json = Get-Content $codexAuth -Raw | ConvertFrom-Json
            if ($json.tokens.access_token) {
                return @{ url = "https://api.openai.com/v1/chat/completions"; model = "gpt-4o-mini"; key = $json.tokens.access_token; source = "codex-oauth" }
            }
            if ($json.OPENAI_API_KEY) {
                return @{ url = "https://api.openai.com/v1/chat/completions"; model = "gpt-4o-mini"; key = $json.OPENAI_API_KEY; source = "codex-api-key" }
            }
        } catch {}
    }
    if ($env:OPENAI_API_KEY) {
        return @{ url = "https://api.openai.com/v1/chat/completions"; model = "gpt-4o-mini"; key = $env:OPENAI_API_KEY; source = "OPENAI_API_KEY" }
    }
    return $null
}

function Resolve-Anthropic {
    $token = if ($env:ANTHROPIC_OAUTH_TOKEN) { $env:ANTHROPIC_OAUTH_TOKEN } elseif ($env:ANTHROPIC_API_KEY) { $env:ANTHROPIC_API_KEY } else { $null }
    if ($token) {
        return @{ url = "https://api.anthropic.com/v1/chat/completions"; model = "claude-3-5-sonnet-latest"; key = $token; source = "anthropic-env" }
    }
    return $null
}

function Resolve-Ollama {
    try {
        $null = Invoke-RestMethod -Method Get -Uri "http://127.0.0.1:11434/api/tags" -TimeoutSec 2
        return @{ url = "http://127.0.0.1:11434/v1/chat/completions"; model = "llama3.2"; key = "ollama"; source = "local-ollama" }
    } catch {
        return $null
    }
}

$resolved = switch ($Provider) {
    "gemini"    { Resolve-Gemini }
    "openai"    { Resolve-OpenAI }
    "anthropic" { Resolve-Anthropic }
    "ollama"    { Resolve-Ollama }
    "auto"      {
        $r = Resolve-Gemini
        if (-not $r) { $r = Resolve-OpenAI }
        if (-not $r) { $r = Resolve-Anthropic }
        if (-not $r) { $r = Resolve-Ollama }
        $r
    }
}

if (-not $resolved) {
    Write-Error "No OAuth session or API key found for provider '$Provider'. Sign in with 'gcloud auth login' or set GEMINI_API_KEY / OPENAI_API_KEY / ANTHROPIC_API_KEY."
    exit 1
}

$env:QUESTFORGE_LLM_URL = $resolved.url
$env:QUESTFORGE_LLM_MODEL = $resolved.model
$env:QUESTFORGE_LLM_API_KEY = $resolved.key

Write-Host "Configured QuestForge LLM ($($resolved.source)):"
Write-Host "  QUESTFORGE_LLM_URL   = $($resolved.url)"
Write-Host "  QUESTFORGE_LLM_MODEL = $($resolved.model)"
