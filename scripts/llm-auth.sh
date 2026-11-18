#!/usr/bin/env bash
set -euo pipefail

# Usage: source scripts/llm-auth.sh [auto|gemini|openai|anthropic|ollama]
PROVIDER="${1:-auto}"

resolve_gemini() {
  if [[ -n "${GEMINI_API_KEY:-}" ]]; then
    QUESTFORGE_LLM_URL="https://generativelanguage.googleapis.com/v1beta/openai/chat/completions"
    QUESTFORGE_LLM_MODEL="gemini-flash-latest"
    QUESTFORGE_LLM_API_KEY="${GEMINI_API_KEY}"
    return 0
  fi
  if command -v gcloud >/dev/null 2>&1; then
    local proj uid key
    proj="$(gcloud projects list --filter="projectId:gen-lang-client*" --format="value(projectId)" --limit=1 2>/dev/null || true)"
    if [[ -n "$proj" ]]; then
      uid="$(gcloud services api-keys list --project "$proj" --filter="restrictions.apiTargets.service:generativelanguage.googleapis.com" --format="value(uid)" --limit=1 2>/dev/null || true)"
      if [[ -n "$uid" ]]; then
        key="$(gcloud services api-keys get-key-string "$uid" --project "$proj" --format="value(keyString)" 2>/dev/null || true)"
        if [[ -n "$key" ]]; then
          QUESTFORGE_LLM_URL="https://generativelanguage.googleapis.com/v1beta/openai/chat/completions"
          QUESTFORGE_LLM_MODEL="gemini-flash-latest"
          QUESTFORGE_LLM_API_KEY="$key"
          return 0
        fi
      fi
    fi
  fi
  if [[ -n "${GOOGLE_API_KEY:-}" ]]; then
    QUESTFORGE_LLM_URL="https://generativelanguage.googleapis.com/v1beta/openai/chat/completions"
    QUESTFORGE_LLM_MODEL="gemini-flash-latest"
    QUESTFORGE_LLM_API_KEY="${GOOGLE_API_KEY}"
    return 0
  fi
  return 1
}

resolve_openai() {
  local token="${OPENAI_OAUTH_TOKEN:-${OPENAI_API_KEY:-}}"
  if [[ -n "$token" ]]; then
    QUESTFORGE_LLM_URL="https://api.openai.com/v1/chat/completions"
    QUESTFORGE_LLM_MODEL="gpt-4o-mini"
    QUESTFORGE_LLM_API_KEY="$token"
    return 0
  fi
  return 1
}

resolve_anthropic() {
  local token="${ANTHROPIC_OAUTH_TOKEN:-${ANTHROPIC_API_KEY:-}}"
  if [[ -n "$token" ]]; then
    QUESTFORGE_LLM_URL="https://api.anthropic.com/v1/chat/completions"
    QUESTFORGE_LLM_MODEL="claude-3-5-sonnet-latest"
    QUESTFORGE_LLM_API_KEY="$token"
    return 0
  fi
  return 1
}

resolve_ollama() {
  if curl -fsS --max-time 2 "http://127.0.0.1:11434/api/tags" >/dev/null 2>&1; then
    QUESTFORGE_LLM_URL="http://127.0.0.1:11434/v1/chat/completions"
    QUESTFORGE_LLM_MODEL="llama3.2"
    QUESTFORGE_LLM_API_KEY="ollama"
    return 0
  fi
  return 1
}

case "$PROVIDER" in
  gemini) resolve_gemini ;;
  openai) resolve_openai ;;
  anthropic) resolve_anthropic ;;
  ollama) resolve_ollama ;;
  auto)
    resolve_gemini || resolve_openai || resolve_anthropic || resolve_ollama
    ;;
  *)
    echo "Unknown provider: $PROVIDER" >&2
    return 1 2>/dev/null || exit 1
    ;;
esac

export QUESTFORGE_LLM_URL QUESTFORGE_LLM_MODEL QUESTFORGE_LLM_API_KEY
echo "Configured QuestForge LLM: ${QUESTFORGE_LLM_MODEL} (${QUESTFORGE_LLM_URL})"
