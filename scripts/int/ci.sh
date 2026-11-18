#!/usr/bin/env bash
# Software Architecture Framework (swarch) - Integration CI Script
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

# 1. Architecture Audit Gate
log_info "Step 1/4: Running architectural audit (swartch)..."
swartch --vocabulary config/vocabulary.toml \
    crates/questforge-api \
    crates/questforge-ci \
    crates/questforge-config \
    crates/questforge-toolchain \
    crates/questforge-doctor \
    crates/questforge-init \
    apps/quest

# 2. Formatting & Lint Gate
log_info "Step 2/4: Checking formatting and clippy..."
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings

# 3. Workspace Check Gate
log_info "Step 3/4: Checking workspace targets..."
cargo check --workspace --all-targets

# 4. Workspace Test Gate
log_info "Step 4/4: Running workspace test suite..."
cargo test --workspace

printf "\n${BOLD}${GREEN}CI Integration Gate: ALL CHECKS PASSED.${RESET}\n"
