#!/usr/bin/env bash
# QuestForge - Integration CI Script
# Vendor-agnostic local/offline integration gate for CI verification

set -euo pipefail

if [[ -d "${HOME}/.cargo/bin" ]]; then
    export PATH="${HOME}/.cargo/bin:${PATH}"
fi

BOLD="\033[1m"
GREEN="\033[0;32m"
RED="\033[0;31m"
RESET="\033[0m"

log_info() { printf "${GREEN}[INFO]${RESET} %s\n" "$1"; }
log_error() { printf "${RED}[ERROR]${RESET} %s\n" "$1" >&2; }

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${REPO_ROOT}"

printf "${BOLD}=== Executing Vendor-Agnostic CI Integration Suite ===${RESET}\n\n"

# 1. Formatting & Lint Gate
log_info "Step 1/3: Checking formatting and clippy..."
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings

# 2. Workspace Check Gate
log_info "Step 2/3: Checking workspace targets..."
cargo check --workspace --all-targets

# 3. Workspace Test Gate
log_info "Step 3/3: Running workspace test suite..."
cargo test --workspace

printf "\n${BOLD}${GREEN}CI Integration Gate: ALL CHECKS PASSED.${RESET}\n"
