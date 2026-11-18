#!/usr/bin/env bash
# Software Architecture Framework (swarch) - testing Environment Install Script
# Compiles and installs quest binary for testing environment

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

log_info "Installing quest CLI binary (testing mode)..."

cargo install --path apps/quest --debug --force

if command -v quest >/dev/null 2>&1; then
    log_info "Successfully installed quest at $(which quest)"
    quest --help >/dev/null 2>&1 || true
    printf "${BOLD}${GREEN}quest CLI is ready for testing!${RESET}\n"
else
    log_error "Installation failed or quest is not in PATH."
    exit 1
fi

