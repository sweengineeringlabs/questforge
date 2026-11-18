#!/usr/bin/env bash
# Software Architecture Framework (swarch) - local Environment Release Script
# Prepares and validates release artifacts for local environment

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

printf "${BOLD}=== Executing local release cycle for quest ===${RESET}\n\n"

log_info "Running validation check for local..."
cargo check
cargo test

log_info "local release cycle completed successfully."
