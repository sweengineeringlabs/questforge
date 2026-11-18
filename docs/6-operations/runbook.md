# QuestForge Operations Runbook

Operational procedures for running the QuestForge control plane, `machineweb` (`mweb`) dashboard, lab fleet agent, and end-to-end verification.

---

## 1. Prerequisites

- **Rust:** `1.100.0` (pinned in `rust-toolchain.toml`)
- **Architecture Auditor:** `swartch` CLI on `PATH`
- **Frontend Toolchain:** Bun `1.4.2` + `mweb` CLI (`machineweb`)
- **Optional Production Services:** PostgreSQL (when using `--database-url`) and Android `platform-tools` (`adb`) on lab hosts

---

## 2. Build & Verify Locally

### Rust Workspace & Architecture Gate
```pwsh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
swartch --vocabulary config/vocabulary.toml crates/questforge-api crates/questforge-ci crates/questforge-config crates/questforge-toolchain crates/questforge-verify crates/questforge-doctor crates/questforge-init crates/questforge-mcp crates/questforge-ops crates/questforge-server apps/quest
```

### Web Frontend (`web/`)
```pwsh
Push-Location web
bun install
mweb generate app
mweb check --browser
bun run build
bun run test
Pop-Location
```

---

## 3. Running the Control Plane (`quest serve`)

### Local Development Mode (In-Memory Store, Header Role Auth)
```pwsh
cargo run --bin quest -- serve --addr 127.0.0.1:8090 --web-root web/dist
```

### Production Mode (PostgreSQL + HS256 JWT Auth)
```pwsh
$env:QUESTFORGE_DATABASE_URL = "postgres://user:password@localhost:5432/questforge"
$env:QUESTFORGE_JWT_SECRET   = "<strong-random-secret>"
quest serve --addr 0.0.0.0:8080 --web-root web/dist
```

Exposed endpoints:
- `GET /` — `machineweb` dashboard (`web/dist/index.html`)
- `GET /health` — liveness check (`{"status":"ok"}`)
- `GET /metrics` — Prometheus counters (`questforge_*_total`)
- `GET|POST /api/projects`, `/api/builds`, `/api/verify`, `/api/reports/{id}`, `/api/deployments`, `/api/audit`

---

## 4. End-to-End Smoke Verification (`scripts/e2e-demo.ps1`)

With `quest serve` running on `http://127.0.0.1:8090`:
```pwsh
pwsh -File scripts/e2e-demo.ps1
```

This script exercises the full stack end-to-end:
1. Scaffolds a sample Unity Quest project (`quest init`)
2. Runs workstation diagnostics (`quest doctor --json`)
3. Generates build metadata and CycloneDX SBOM (`quest ci --dry-run --json`)
4. Builds and audits a signed ARM64 Quest APK fixture (`quest verify --format json`)
5. Publishes the project, build, verification scorecard, and deployment to `http://127.0.0.1:8090/api/*`

---

## 5. Running the Lab Fleet Agent (`quest ops agent`)

On a lab host with Quest headsets or an Android emulator (`../mobile-emulator`) connected via `adb`:
```pwsh
quest ops agent --server http://127.0.0.1:8090 --group qa-lab
```

---

## 6. Troubleshooting

| Symptom | Cause | Resolution |
|---|---|---|
| `os error 10048` on `quest serve` | Port already in use | Pass `--addr 127.0.0.1:<free-port>` |
| Blank page or CSS/JS `404` on `/` | `web/dist/` not built or `--web-root` points to `web/` instead of `web/dist` | Run `bun run build` in `web/` and pass `--web-root web/dist` |
| `native TypeScript is missing` in `mweb check` | Dependencies not installed in `web/` | Run `bun install` inside `web/` |
| `MissingChecksum` in `quest setup` | Zero-trust policy requires SHA-256 | Provide `--adb-sha256` / `--ovr-sha256` (or `quest_forge.yaml` `*_sha256` fields) |
