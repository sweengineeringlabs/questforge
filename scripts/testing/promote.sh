#!/usr/bin/env bash
# Software Architecture Framework (swarch) - testing Environment Promotion Script
# Validates gate conditions and promotes artifacts for testing environment

set -euo pipefail

if [[ -d "${HOME}/.cargo/bin" ]]; then
    export PATH="${HOME}/.cargo/bin:${PATH}"
fi

BOLD="\033[1m"
GREEN="\033[0;32m"
YELLOW="\033[0;33m"
RED="\033[0;31m"
RESET="\033[0m"

log_info() { printf "${GREEN}[INFO]${RESET} %s\n" "$1"; }
log_warn() { printf "${YELLOW}[WARN]${RESET} %s\n" "$1"; }
log_error() { printf "${RED}[ERROR]${RESET} %s\n" "$1" >&2; }

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${REPO_ROOT}"

printf "${BOLD}=== Promoting swarch for testing ===${RESET}\n\n"

if [[ -n "$(git status --porcelain)" ]]; then
    log_error "Working tree has uncommitted changes. Stash or commit before promoting."
    exit 1
fi

log_info "Running verification suite..."
cargo test

log_info "testing promotion gate passed."
printf "${BOLD}${GREEN}testing promotion gate verified successfully!${RESET}\n"
