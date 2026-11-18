#!/usr/bin/env bash
set -euo pipefail

quest setup --project-dir .
quest sync --project-dir .
quest check --project-dir .
quest doctor --ci --project-dir .
