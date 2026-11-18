#!/usr/bin/env bash
# Software Architecture Framework (swarch) - Bootstrap Script
# Sets up development and deployment prerequisites for all environments

set -euo pipefail

BOLD="\033[1m"
GREEN="\033[0;32m"
RED="\033[0;31m"
RESET="\033[0m"

log_info() { printf "${GREEN}[INFO]${RESET} %s\n" "$1"; }
log_error() { printf "${RED}[ERROR]${RESET} %s\n" "$1" >&2; }

log_info "Bootstrapping swarch environment..."

if ! command -v cargo >/dev/null 2>&1; then
    log_error "Cargo is required but not installed. Please install Rust toolchain."
    exit 1
fi

log_info "Rust toolchain detected: $(cargo --version)"
log_info "Bootstrap complete."
