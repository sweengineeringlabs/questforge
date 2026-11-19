# QuestForge Operations Runbook

**Audience**: Platform & DevOps Engineers, XR QA & Release Managers, and Enterprise Lab Operations Leads

> **TLDR**: Operational procedures for installing `quest` on Windows and Linux / WSL, building and verifying the workspace and `machineweb` (`mweb`) dashboard, running the `quest serve` Control Plane (in-memory or PostgreSQL + HS256 JWT), operating lab headset fleets (`quest ops`), pushing verified builds to Meta Horizon Store (`quest ops store`) and enterprise MDMs (`quest ops hms-push`), and troubleshooting common errors.

---

## Table of Contents

- [1. Prerequisites & Installation](#1-prerequisites--installation)
- [2. Build & Verify Locally](#2-build--verify-locally)
- [3. Running the Control Plane (`quest serve`)](#3-running-the-control-plane-quest-serve)
- [4. End-to-End Smoke Verification (`scripts/e2e-demo.ps1`)](#4-end-to-end-smoke-verification-scriptse2e-demops1)
- [5. Running the Lab Fleet Agent & Enterprise MDM Push (`quest ops`)](#5-running-the-lab-fleet-agent--enterprise-mdm-push-quest-ops)
- [6. Troubleshooting](#6-troubleshooting)

---

## 1. Prerequisites & Installation

- **Rust:** `1.100.0` (pinned in `rust-toolchain.toml`)
- **Frontend Toolchain:** Bun `1.4.2` + `mweb` CLI (`machineweb`)
- **Optional Production Services:** PostgreSQL (when using `--database-url`) and Android `platform-tools` (`adb`) on lab hosts

### Install Release Binary

#### Windows (PowerShell)
```pwsh
Invoke-WebRequest -Uri "https://github.com/sweengineeringlabs/questforge/releases/latest/download/quest-x86_64-pc-windows-msvc.zip" -OutFile quest.zip
Expand-Archive -Path quest.zip -DestinationPath . -Force
.\quest.exe --version
```

#### Linux & WSL (Bash)
```bash
curl -fsSL https://github.com/sweengineeringlabs/questforge/releases/latest/download/quest-x86_64-unknown-linux-gnu.tar.gz | tar -xz
chmod +x ./quest
./quest --version
```

---

## 2. Build & Verify Locally

### Rust Workspace Verification (Windows & Linux)
```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

### Web Frontend (`web/`)

#### Windows (PowerShell)
```pwsh
Push-Location web
bun install
mweb generate app
mweb check --browser
bun run build
bun run test
Pop-Location
```

#### Linux & WSL (Bash)
```bash
(cd web && bun install && mweb generate app && mweb check --browser && bun run build && bun run test)
```

---

## 3. Running the Control Plane (`quest serve`)

### Local Development Mode (In-Memory Store, Header Role Auth)
```bash
cargo run --bin quest -- serve --addr 127.0.0.1:8090 --web-root web/dist
```

### Production Mode (PostgreSQL + HS256 JWT Auth)

#### Windows (PowerShell)
```pwsh
$env:QUESTFORGE_DATABASE_URL = "postgres://user:password@localhost:5432/questforge"
$env:QUESTFORGE_JWT_SECRET   = "<strong-random-secret>"
quest serve --addr 0.0.0.0:8080 --web-root web/dist
```

#### Linux & WSL (Bash)
```bash
export QUESTFORGE_DATABASE_URL="postgres://user:password@localhost:5432/questforge"
export QUESTFORGE_JWT_SECRET="<strong-random-secret>"
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

### Showcase Game Playbook (`examples/orbit-pulse-vr`)

#### Evaluator Cloud Fast-Track (Zero Toolchain / Instant Verification)
Judges and evaluators can verify the release APK directly from GitHub releases without building from source:
```pwsh
Invoke-WebRequest -Uri "https://github.com/sweengineeringlabs/questforge/releases/latest/download/orbit-pulse-vr-1.0.0.apk" -OutFile orbit-pulse-vr.apk
quest verify orbit-pulse-vr.apk --project-dir examples/orbit-pulse-vr
```

#### Build, Test & Publish from Source
With `quest serve --addr 127.0.0.1:8090 --web-root web/dist` running:
```pwsh
cd examples/orbit-pulse-vr
quest sync
quest check
cargo test
quest ci --output-dir dist --release-version 1.0.0
quest verify dist/orbit-pulse-vr-1.0.0.apk
.\scripts\publish-demo.ps1    # or `scripts\publish-demo.cmd` in cmd.exe
```
Open `http://127.0.0.1:8090/` to inspect `orbit-pulse-vr`, its build, its `100/100` VRC scorecard, and the audit log.

#### Testing the Hand-Wave Interaction (ADR-002)
* **On Meta Quest Headset:** Optical 3D hand tracking (`XR_EXT_hand_tracking`) evaluates palm lateral velocity ($v_x > 0.20\text{ m/s}$) via `OpenXrHandWaveDetector` at 120 Hz with zero heap allocations.
* **On Any Android Phone / Tablet:** Install the APK. Wave a physical hand across the front IR proximity sensor ($< 5\text{ cm}$) to trigger the wave hands-free and boost kinetic shields ($+20\%$).
* **On Headless Emulator / CI:** Swipe across the screen ($|v_x| > 500\text{ px/s}$) or tap **"Wave Hand Gesture"** to trigger simulated gesture dispatch.

---

## 5. Running the Lab Fleet Agent & Enterprise MDM Push (`quest ops`)

QuestForge complements enterprise MDMs (Meta HMS, ManageXR, ArborXR): `quest ops` manages engineering/QA lab headsets over ADB, while `quest ops hms-push` promotes verified APKs into enterprise MDMs for production OTA kiosk rollout.

On a lab host with Quest headsets or an Android emulator (`../mobile-emulator`) connected via `adb`:
```pwsh
quest ops agent --server http://127.0.0.1:8090 --group qa-lab
```

Push a verified `100/100` APK to Meta HMS (`graph.oculus.com`), ManageXR (`managexrapi.com`), or ArborXR (`api.xrdm.app`):

#### Windows (PowerShell)
```pwsh
$env:QUESTFORGE_MDM_TOKEN = "<mdm-bearer-token>"
# Using explicit CLI flags:
quest ops hms-push --provider meta-hms --app-id com.studio.app --apk dist/orbit-pulse-vr-1.0.0.apk --channel PRODUCTION --group training-floor
# Or using `mdm:` settings from quest_forge.yaml:
quest ops hms-push --project-dir examples/orbit-pulse-vr --apk examples/orbit-pulse-vr/dist/orbit-pulse-vr-1.0.0.apk
```

#### Linux & WSL (Bash)
```bash
export QUESTFORGE_MDM_TOKEN="<mdm-bearer-token>"
quest ops hms-push --provider meta-hms --app-id com.studio.app --apk dist/orbit-pulse-vr-1.0.0.apk --channel PRODUCTION --group training-floor
quest ops hms-push --project-dir examples/orbit-pulse-vr --apk examples/orbit-pulse-vr/dist/orbit-pulse-vr-1.0.0.apk
```

### AI Agents & Meta Horizon Store (`quest auth`, `quest agent --role`, `quest ops store`)
Authenticate or inspect OAuth/API-key status (`gemini`, `openai`, `anthropic`):
```bash
quest auth --status
quest auth --provider gemini
```
Run a role-based AI XR agent (`studio`, `enterprise`, `platform`, `qa`, `developer`) over 14 MCP tools:
```bash
quest agent --role qa --provider gemini --auth --prompt "Verify examples/orbit-pulse-vr/dist/orbit-pulse-vr-1.0.0.apk and upload to ALPHA for app_id <APP_ID>"
```
Or run any 1:1 `ovr-platform-util` Store operation directly (`upload-quest-build`, `download-quest-build`, `get-release-channel-data`, `set-release-channel-build`, `download-asset-file`, `version`):

#### Windows (PowerShell)
```pwsh
$env:QUEST_META_APP_SECRET = "<your-meta-app-secret>"
quest ops store --action version
quest ops store --action upload-quest-build --app-id <APP_ID> --apk dist/app.apk --channel ALPHA
```

#### Linux & WSL (Bash)
Meta does not publish a native Linux `ovr-platform-util` binary. When running the Linux `quest` binary inside **WSL (Windows Subsystem for Linux)**:
- **Auto-detection:** `quest` detects WSL automatically via `WSL_DISTRO_NAME`, `WSL_INTEROP`, or `/proc/version`.
- **Toolchain setup:** `quest setup --ovr-version latest` installs native Linux `adb` and `cmdline-tools`, and provisions Meta's Windows `ovr-platform-util.exe` with executable permissions (`0755`) for WSL interop.
- **Zero-config Horizon Store uploads:** `quest ops store` and the MCP `ovr_store` tool automatically check both `~/.questforge/` and the Windows host cache (`/mnt/c/Users/*/.questforge/`), and translate Linux `--apk` / `--dir` paths with `wslpath -w` before invoking `ovr-platform-util.exe`:
```bash
export QUEST_META_APP_SECRET="<your-meta-app-secret>"
quest ops store --action version
quest ops store --action upload-quest-build --app-id <APP_ID> --apk dist/app.apk --channel ALPHA
```
- **USB Quest Headsets on WSL2:** Connect headsets over wireless ADB (`adb connect <ip>:5555`) or attach USB via `usbipd-win` (`usbipd attach --wsl --busid <busid>`).

---

## 6. Troubleshooting

| Symptom | Cause | Resolution |
|---|---|---|
| `os error 10048` on `quest serve` | Port already in use | Pass `--addr 127.0.0.1:<free-port>` |
| `/` shows only `"Powered by machineweb (mweb) components..."` | `quest serve` started without `--web-root` (serving `FALLBACK_DASHBOARD_HTML`) | Restart with `--web-root web/dist` (e.g. `quest serve --addr 127.0.0.1:8090 --web-root C:\phd-systems\swelabs\metaquest\web\dist`) |
| Blank page or CSS/JS `404` on `/` | `web/dist/` not built or `--web-root` points to `web/` instead of `web/dist` | Run `bun run build` in `web/` and pass `--web-root web/dist` |
| `401 {"error": "missing X-Quest-Role"}` on `POST` | Missing RBAC role header in DevHeader auth mode | Pass `-H "X-Quest-Role: admin"` (or `developer` / `platform_engineer` / `operations_engineer`) |
| `404` on `POST /api/reports` | Write endpoint for verification scorecards is `POST /api/verify` | Post verification results to `POST /api/verify`; read via `GET /api/reports` |
| `native TypeScript is missing` in `mweb check` | Dependencies not installed in `web/` | Run `bun install` inside `web/` |
| `MissingChecksum` in `quest setup` | Zero-trust policy requires SHA-256 | Provide `--adb-sha256` / `--ovr-sha256` (or `quest_forge.yaml` `*_sha256` fields) |

---

## See Also

- [Documentation Hub (`docs/README.md`)](../README.md)
- [Quick Start Guide](../quickstart.md)
- [Architecture](../3-design/architecture.md)
- [Role Workflows](../3-design/workflow/README.md)
- [Developer Guide](../4-development/developer_guide.md)
