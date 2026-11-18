//! CLI execution coordination for `quest`.

use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand, ValueEnum};
use questforge_ci::{CiOptions, CiOrchestrator, CiProvider};
use questforge_config::{EngineKind, MdmTarget, QuestforgeConfig};
use questforge_doctor::{CheckStatus, DoctorOptions, DoctorRunner};
use questforge_init::{InitOptions, ProjectScaffolder};
use questforge_mcp::{
    AgentGoal, AgentOptions, AgentRole, AutoLlmCredentialResolver, LlmEndpointConfig,
    QuestMcpServer, QuestXrAgent,
};
use questforge_ops::{
    FleetInventory, FleetManager, HmsReleaseRequest, MdmProvider, OpsEnv, OvrStoreAction,
    OvrStoreRequest, StoreUploadRequest,
};
use questforge_server::{AuthMode, ControlPlaneServer, JwtVerifier, PostgresStore, StoreBackend};
use questforge_toolchain::{SetupOptions, ToolchainProvisioner};
use questforge_verify::{ApkVerifier, ReportFormat, VerifyOptions};

#[derive(Debug, Parser)]
#[command(
    name = "quest",
    version,
    about = "QuestForge CLI for Meta Quest and Meta Horizon OS development"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Install and verify Quest toolchain binaries (adb, cmdline-tools, ovr-platform-util)
    Setup(SetupArgs),
    /// Validate developer workstation or CI environment against quest_forge.yaml
    Doctor(DoctorArgs),
    /// Scaffold a Quest-ready Unity or Unreal repository
    Init(InitArgs),
    /// Synchronize declarative quest_forge.yaml capabilities into engine manifests and bootstrap code
    Sync(SyncArgs),
    /// Run sub-second static source, VRC manifest, and XR frame-budget checks
    Check(CheckArgs),
    /// Build, package, generate SBOM, and publish Quest APK artifacts
    Ci(CiArgs),
    /// Verify APK release readiness, signatures, ABIs, and Meta VRC policy compliance
    Verify(VerifyArgs),
    /// Run the QuestForge Model Context Protocol (MCP) server over stdio
    Mcp(McpArgs),
    /// Run the coded autonomous QuestForge XR DevOps agent over live MCP stdio
    Agent(AgentArgs),
    /// Authenticate or inspect OAuth/API-key credentials for LLM providers (`gemini`, `openai`, `anthropic`)
    Auth(AuthArgs),
    /// Manage Quest lab and device fleets (discover, deploy, rollback, telemetry, logcat, screenshot)
    Ops(OpsArgs),
    /// Run the QuestForge hosted control plane HTTP server
    Serve(ServeArgs),
}

#[derive(Debug, Args)]
struct SetupArgs {
    /// Directory containing quest_forge.yaml (defaults to current directory)
    #[arg(long, default_value = ".")]
    project_dir: PathBuf,
    /// Override Android platform-tools (adb) version
    #[arg(long)]
    adb_version: Option<String>,
    /// Expected SHA-256 hex digest for adb archive
    #[arg(long)]
    adb_sha256: Option<String>,
    /// Install Android cmdline-tools with the specified version
    #[arg(long)]
    cmdline_tools_version: Option<String>,
    /// Expected SHA-256 hex digest for cmdline-tools archive
    #[arg(long)]
    cmdline_tools_sha256: Option<String>,
    /// Install ovr-platform-util with the specified version
    #[arg(long)]
    ovr_version: Option<String>,
    /// Download URL for ovr-platform-util
    #[arg(long)]
    ovr_url: Option<String>,
    /// Expected SHA-256 hex digest for ovr-platform-util
    #[arg(long)]
    ovr_sha256: Option<String>,
    /// Allow downloading archives without a SHA-256 checksum (disables zero-trust check)
    #[arg(long)]
    allow_unverified: bool,
    /// Override cache root (defaults to ~/.questforge/cache)
    #[arg(long)]
    cache_dir: Option<PathBuf>,
    /// Emit machine-readable JSON output
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct DoctorArgs {
    /// Directory containing quest_forge.yaml (defaults to current directory)
    #[arg(long, default_value = ".")]
    project_dir: PathBuf,
    /// CI mode (ignores missing USB headset; enforces signing and swartch gate)
    #[arg(long)]
    ci: bool,
    /// Strict mode (treats signing warnings as failures)
    #[arg(long)]
    strict: bool,
    /// Automatically synchronize and heal drifted project manifests/configs
    #[arg(long)]
    fix: bool,
    /// Emit machine-readable JSON output
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct SyncArgs {
    /// Directory containing quest_forge.yaml (defaults to current directory)
    #[arg(long, default_value = ".")]
    project_dir: PathBuf,
    /// Emit machine-readable JSON output
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct CheckArgs {
    /// Directory containing quest_forge.yaml (defaults to current directory)
    #[arg(long, default_value = ".")]
    project_dir: PathBuf,
    /// Emit machine-readable JSON output
    #[arg(long)]
    json: bool,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CliEngine {
    Unity,
    Unreal,
    Rust,
}

impl From<CliEngine> for EngineKind {
    fn from(value: CliEngine) -> Self {
        match value {
            CliEngine::Unity => Self::Unity,
            CliEngine::Unreal => Self::Unreal,
            CliEngine::Rust => Self::Rust,
        }
    }
}

#[derive(Debug, Args)]
struct InitArgs {
    /// Target directory / project name to create
    name: PathBuf,
    /// Target XR game engine
    #[arg(long, value_enum, default_value_t = CliEngine::Unity)]
    engine: CliEngine,
    /// Pin engine version (e.g. 2022.3.42f1 or 5.4)
    #[arg(long)]
    engine_version: Option<String>,
    /// Android reverse-domain package ID
    #[arg(long)]
    package_id: Option<String>,
    /// Optional external Git template URL
    #[arg(long)]
    template: Option<String>,
    /// Overwrite existing files in a non-empty directory
    #[arg(long)]
    force: bool,
    /// Skip automatic post-init `quest doctor` check
    #[arg(long)]
    skip_doctor: bool,
    /// Emit machine-readable JSON output
    #[arg(long)]
    json: bool,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CliCiProvider {
    Github,
    Azure,
    Jenkins,
}

impl From<CliCiProvider> for CiProvider {
    fn from(value: CliCiProvider) -> Self {
        match value {
            CliCiProvider::Github => Self::Github,
            CliCiProvider::Azure => Self::Azure,
            CliCiProvider::Jenkins => Self::Jenkins,
        }
    }
}

#[derive(Debug, Args)]
struct CiArgs {
    /// Directory containing quest_forge.yaml (defaults to current directory)
    #[arg(long, default_value = ".")]
    project_dir: PathBuf,
    /// Output directory for APK, build-metadata.json, and sbom.cdx.json
    #[arg(long, default_value = "dist")]
    output_dir: PathBuf,
    /// Release version string
    #[arg(long, default_value = "0.1.0")]
    release_version: String,
    /// Optional Meta Horizon Store channel (e.g. ALPHA, RC, LIVE)
    #[arg(long)]
    upload_channel: Option<String>,
    /// Optional Meta App ID for ovr-platform-util upload
    #[arg(long)]
    app_id: Option<String>,
    /// Emit a starter CI pipeline template instead of running a build
    #[arg(long, value_enum)]
    emit_pipeline: Option<CliCiProvider>,
    /// Plan build, generate metadata and SBOM without executing engine build
    #[arg(long)]
    dry_run: bool,
    /// Emit machine-readable JSON output
    #[arg(long)]
    json: bool,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CliReportFormat {
    Cli,
    Json,
    Sarif,
    Html,
    Pdf,
}

impl From<CliReportFormat> for ReportFormat {
    fn from(value: CliReportFormat) -> Self {
        match value {
            CliReportFormat::Cli => Self::Cli,
            CliReportFormat::Json => Self::Json,
            CliReportFormat::Sarif => Self::Sarif,
            CliReportFormat::Html => Self::Html,
            CliReportFormat::Pdf => Self::Pdf,
        }
    }
}

#[derive(Debug, Args)]
struct VerifyArgs {
    /// Path to the APK artifact to verify
    apk: PathBuf,
    /// Optional directory containing quest_forge.yaml
    #[arg(long)]
    project_dir: Option<PathBuf>,
    /// Output report format
    #[arg(long, value_enum, default_value_t = CliReportFormat::Cli)]
    format: CliReportFormat,
    /// Write report output to a file instead of stdout
    #[arg(long)]
    out: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct McpArgs {
    /// Workspace root used for documentation search
    #[arg(long, default_value = ".")]
    workspace_root: PathBuf,
    /// Print available MCP tool definitions as JSON and exit
    #[arg(long)]
    list_tools: bool,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CliAgentGoal {
    Check,
    Release,
    FleetRelease,
}

impl From<CliAgentGoal> for AgentGoal {
    fn from(value: CliAgentGoal) -> Self {
        match value {
            CliAgentGoal::Check => Self::Check,
            CliAgentGoal::Release => Self::Release,
            CliAgentGoal::FleetRelease => Self::FleetRelease,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
enum CliAgentRole {
    Studio,
    Enterprise,
    Platform,
    Qa,
    #[default]
    Developer,
}

impl From<CliAgentRole> for AgentRole {
    fn from(value: CliAgentRole) -> Self {
        match value {
            CliAgentRole::Studio => Self::Studio,
            CliAgentRole::Enterprise => Self::Enterprise,
            CliAgentRole::Platform => Self::Platform,
            CliAgentRole::Qa => Self::Qa,
            CliAgentRole::Developer => Self::Developer,
        }
    }
}

#[derive(Debug, Args)]
struct AgentArgs {
    /// Target QuestForge project directory
    #[arg(long, default_value = ".")]
    project_dir: PathBuf,
    /// Output directory for APK, metadata, and SBOM
    #[arg(long, default_value = "dist")]
    output_dir: PathBuf,
    /// Release version string
    #[arg(long, default_value = "1.0.0")]
    release_version: String,
    /// Autonomous goal to execute (defaults to role default or `release`)
    #[arg(long, value_enum)]
    goal: Option<CliAgentGoal>,
    /// Agent role profile (`studio`, `enterprise`, `platform`, `qa`, `developer`)
    #[arg(long, alias = "audience", value_enum, default_value_t = CliAgentRole::Developer)]
    role: CliAgentRole,
    /// Optional natural-language prompt to run in LLM ReAct mode via `quest mcp`
    #[arg(long)]
    prompt: Option<String>,
    /// LLM provider SPI (`gemini`, `openai`, `anthropic`; auto-detected when omitted)
    #[arg(long)]
    provider: Option<String>,
    /// Run OAuth login hook for the selected provider when credentials are missing
    #[arg(long)]
    auth: bool,
    /// OpenAI-compatible chat completions URL (or QUESTFORGE_LLM_URL env var)
    #[arg(long)]
    llm_url: Option<String>,
    /// LLM model name (or QUESTFORGE_LLM_MODEL env var)
    #[arg(long)]
    model: Option<String>,
    /// Environment variable holding the LLM Bearer API key
    #[arg(long, default_value = "QUESTFORGE_LLM_API_KEY")]
    api_key_env: String,
    /// Maximum ReAct turns in LLM mode
    #[arg(long, default_value_t = 8)]
    max_turns: usize,
    /// Plan build without invoking engine compiler
    #[arg(long)]
    dry_run: bool,
    /// Emit machine-readable JSON output
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct AuthArgs {
    /// Target LLM provider (`gemini`, `openai`, `anthropic`; defaults to `gemini`)
    #[arg(long, default_value = "gemini")]
    provider: String,
    /// Force re-authentication even if cached OAuth credentials exist
    #[arg(long)]
    force: bool,
    /// Print credential status for all built-in providers without launching login
    #[arg(long)]
    status: bool,
}

#[derive(Debug, Args)]
struct OpsArgs {
    /// Fleet inventory registry JSON path
    #[arg(long, default_value = ".questforge/fleet.json")]
    registry: PathBuf,
    /// Emit JSON output
    #[arg(long)]
    json: bool,
    #[command(subcommand)]
    action: OpsAction,
}

#[derive(Debug, Subcommand)]
enum OpsAction {
    /// Discover connected adb devices and update inventory
    Discover {
        /// Default group for newly discovered devices
        #[arg(long, default_value = "default")]
        group: String,
    },
    /// Register or assign a device serial to a group
    Register {
        /// Device serial or <ip>:<port>
        serial: String,
        /// Target group name
        #[arg(long, default_value = "default")]
        group: String,
    },
    /// List registered devices in inventory
    List,
    /// Refresh firmware and battery telemetry for a device
    Telemetry {
        /// Device serial
        serial: String,
    },
    /// Deploy (or schedule) an APK to a device serial or group
    Deploy {
        /// Target device serial or group name
        target: String,
        /// Path to APK file
        apk: PathBuf,
        /// Optional scheduled timestamp/window
        #[arg(long)]
        schedule: Option<String>,
    },
    /// Roll back a device or group to its previous APK
    Rollback {
        /// Target device serial or group name
        target: String,
    },
    /// Capture logcat output from a device
    Logcat {
        /// Device serial
        serial: String,
        /// Output log file path
        #[arg(long, default_value = "device-logcat.txt")]
        out: PathBuf,
    },
    /// Capture a PNG screenshot from a device
    Screenshot {
        /// Device serial
        serial: String,
        /// Output PNG path
        #[arg(long, default_value = "screenshot.png")]
        out: PathBuf,
    },
    /// Poll questforge-server for scheduled deployments and execute them on local devices
    Agent {
        /// Control plane base URL
        #[arg(long, default_value = "http://127.0.0.1:8080")]
        server: String,
        /// Default group for discovered devices
        #[arg(long, default_value = "qa-lab")]
        group: String,
    },
    /// Publish a verified APK to Meta Horizon Managed Services (HMS) or an enterprise MDM
    HmsPush {
        /// Optional project directory containing quest_forge.yaml (`mdm:` defaults)
        #[arg(long)]
        project_dir: Option<PathBuf>,
        /// Target MDM provider (`meta-hms`, `managexr`, `arborxr`)
        #[arg(long, value_enum)]
        provider: Option<CliMdmProvider>,
        /// Application ID in the MDM
        #[arg(long)]
        app_id: Option<String>,
        /// Path to the verified `.apk`
        #[arg(long)]
        apk: PathBuf,
        /// Release channel or track (e.g. ALPHA, BETA, PRODUCTION)
        #[arg(long)]
        channel: Option<String>,
        /// Target MDM device group
        #[arg(long)]
        group: Option<String>,
        /// Optional custom MDM endpoint URL
        #[arg(long)]
        endpoint: Option<String>,
        /// Environment variable containing the MDM bearer token
        #[arg(long)]
        token_env: Option<String>,
    },
    /// Upload a signed APK directly to a Meta Horizon Store release channel via `ovr-platform-util`
    StoreUpload {
        /// Meta Horizon App ID (e.g. `1439257382612621`)
        #[arg(long)]
        app_id: String,
        /// Path to the signed `.apk`
        #[arg(long)]
        apk: PathBuf,
        /// Target release channel (`ALPHA`, `BETA`, `RC`, `PRODUCTION`)
        #[arg(long, default_value = "ALPHA")]
        channel: String,
        /// Environment variable holding the Meta Horizon App Secret
        #[arg(long, default_value = "QUEST_META_APP_SECRET")]
        app_secret_env: String,
        /// Optional release notes
        #[arg(long)]
        notes: Option<String>,
        /// Optional explicit path to `ovr-platform-util`
        #[arg(long)]
        ovr_bin: Option<PathBuf>,
    },
    /// Execute any 1:1 `ovr-platform-util` Meta Horizon Store operation
    Store {
        /// Store operation (`upload-quest-build`, `upload-dlc-build`, `upload-debug-symbols`, `set-release-channel-build`, `list-builds`, `download-build`)
        #[arg(long, value_enum)]
        action: CliOvrStoreAction,
        /// Meta Horizon App ID
        #[arg(long)]
        app_id: String,
        /// Optional APK path (`upload-quest-build`)
        #[arg(long)]
        apk: Option<PathBuf>,
        /// Optional release channel (`ALPHA`, `BETA`, `RC`, `PRODUCTION`)
        #[arg(long)]
        channel: Option<String>,
        /// Optional build ID (`set-release-channel-build`, `upload-debug-symbols`, `download-build`)
        #[arg(long)]
        build_id: Option<String>,
        /// Optional directory path (debug symbols, DLC assets, or download destination)
        #[arg(long)]
        dir: Option<PathBuf>,
        /// Optional DLC SKU (`upload-dlc-build`)
        #[arg(long)]
        sku: Option<String>,
        /// Optional release notes
        #[arg(long)]
        notes: Option<String>,
        /// Environment variable holding the Meta Horizon App Secret
        #[arg(long, default_value = "QUEST_META_APP_SECRET")]
        app_secret_env: String,
        /// Optional explicit path to `ovr-platform-util`
        #[arg(long)]
        ovr_bin: Option<PathBuf>,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CliOvrStoreAction {
    #[value(name = "upload-quest-build")]
    UploadQuestBuild,
    #[value(name = "download-quest-build")]
    DownloadQuestBuild,
    #[value(name = "get-release-channel-data")]
    GetReleaseChannelData,
    #[value(name = "set-release-channel-build")]
    SetReleaseChannelBuild,
    #[value(name = "download-asset-file")]
    DownloadAssetFile,
    #[value(name = "version")]
    Version,
}

impl From<CliOvrStoreAction> for OvrStoreAction {
    fn from(value: CliOvrStoreAction) -> Self {
        match value {
            CliOvrStoreAction::UploadQuestBuild => Self::UploadQuestBuild,
            CliOvrStoreAction::DownloadQuestBuild => Self::DownloadQuestBuild,
            CliOvrStoreAction::GetReleaseChannelData => Self::GetReleaseChannelData,
            CliOvrStoreAction::SetReleaseChannelBuild => Self::SetReleaseChannelBuild,
            CliOvrStoreAction::DownloadAssetFile => Self::DownloadAssetFile,
            CliOvrStoreAction::Version => Self::Version,
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CliMdmProvider {
    #[value(name = "meta-hms")]
    MetaHms,
    #[value(name = "managexr")]
    ManageXr,
    #[value(name = "arborxr")]
    ArborXr,
}

impl From<CliMdmProvider> for MdmProvider {
    fn from(value: CliMdmProvider) -> Self {
        match value {
            CliMdmProvider::MetaHms => Self::MetaHms,
            CliMdmProvider::ManageXr => Self::ManageXr,
            CliMdmProvider::ArborXr => Self::ArborXr,
        }
    }
}

#[derive(Debug, Args)]
struct ServeArgs {
    /// Listen address (host:port)
    #[arg(long, default_value = "127.0.0.1:8080")]
    addr: String,
    /// Optional directory serving machineweb (mweb) dashboard assets
    #[arg(long)]
    web_root: Option<PathBuf>,
    /// Optional PostgreSQL connection URL (or QUESTFORGE_DATABASE_URL env var)
    #[arg(long)]
    database_url: Option<String>,
    /// Optional HS256 JWT secret (or QUESTFORGE_JWT_SECRET env var)
    #[arg(long)]
    jwt_secret: Option<String>,
}

/// Command-line runner for `quest`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Runner;

impl Runner {
    /// Executes the command-line interface.
    pub(crate) fn run() -> ExitCode {
        let cli = Cli::parse();
        match cli.command {
            Commands::Setup(args) => run_setup(args),
            Commands::Doctor(args) => run_doctor(args),
            Commands::Init(args) => run_init(args),
            Commands::Sync(args) => run_sync(args),
            Commands::Check(args) => run_check(args),
            Commands::Ci(args) => run_ci(args),
            Commands::Verify(args) => run_verify(args),
            Commands::Mcp(args) => run_mcp(args),
            Commands::Agent(args) => run_agent(args),
            Commands::Auth(args) => run_auth(args),
            Commands::Ops(args) => run_ops(args),
            Commands::Serve(args) => run_serve(args),
        }
    }
}

fn run_setup(args: SetupArgs) -> ExitCode {
    let opts = SetupOptions {
        project_dir: Some(args.project_dir),
        adb_version: args.adb_version,
        adb_sha256: args.adb_sha256,
        cmdline_tools_version: args.cmdline_tools_version,
        cmdline_tools_sha256: args.cmdline_tools_sha256,
        ovr_version: args.ovr_version,
        ovr_url: args.ovr_url,
        ovr_sha256: args.ovr_sha256,
        cache_dir: args.cache_dir,
        include_cmdline_tools: false,
        require_checksum: !args.allow_unverified,
    };

    match ToolchainProvisioner::provision(&opts) {
        Ok(report) => {
            if args.json {
                match serde_json::to_string_pretty(&report) {
                    Ok(json) => println!("{json}"),
                    Err(err) => {
                        eprintln!("failed to serialize setup report: {err}");
                        return ExitCode::FAILURE;
                    }
                }
            } else {
                println!(
                    "QuestForge toolchain ready ({}-{}, cache: {})",
                    report.platform.os,
                    report.platform.arch,
                    report.cache_root.display()
                );
                for tool in &report.tools {
                    let status = if tool.cache_hit {
                        "cached"
                    } else {
                        "installed"
                    };
                    println!(
                        "  - {} {} [{}]: {}",
                        tool.tool.dir_name(),
                        tool.version,
                        status,
                        tool.binary_path.display()
                    );
                }
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("quest setup failed: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run_doctor(args: DoctorArgs) -> ExitCode {
    if args.fix {
        let _ = ProjectScaffolder::sync_project(&args.project_dir);
    }
    let opts = DoctorOptions {
        project_dir: args.project_dir,
        ci: args.ci,
        strict: args.strict,
        fix: args.fix,
    };

    match DoctorRunner::run(&opts) {
        Ok(report) => {
            if args.json {
                match serde_json::to_string_pretty(&report) {
                    Ok(json) => println!("{json}"),
                    Err(err) => {
                        eprintln!("failed to serialize doctor report: {err}");
                        return ExitCode::FAILURE;
                    }
                }
            } else {
                for check in &report.checks {
                    let badge = match check.status {
                        CheckStatus::Pass => "PASS",
                        CheckStatus::Warn => "WARN",
                        CheckStatus::Fail => "FAIL",
                    };
                    println!("[{badge}] {} ({}): {}", check.id, check.name, check.details);
                    if let Some(fix) = &check.remediation {
                        println!("       fix: {fix}");
                    }
                }
            }
            if report.has_failures() {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(err) => {
            eprintln!("quest doctor failed: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run_init(args: InitArgs) -> ExitCode {
    let project_name = args
        .name
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("quest-app")
        .to_string();
    let sanitized: String = project_name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect();
    let package_id = args.package_id.unwrap_or_else(|| {
        let suffix = if sanitized.is_empty() {
            "questapp"
        } else {
            sanitized.as_str()
        };
        format!("com.example.{suffix}")
    });

    let opts = InitOptions {
        target_dir: args.name.clone(),
        name: project_name,
        package_id,
        engine: args.engine.into(),
        engine_version: args.engine_version,
        template_url: args.template,
        force: args.force,
    };

    match ProjectScaffolder::scaffold(&opts) {
        Ok(report) => {
            if args.json {
                match serde_json::to_string_pretty(&report) {
                    Ok(json) => println!("{json}"),
                    Err(err) => {
                        eprintln!("failed to serialize init report: {err}");
                        return ExitCode::FAILURE;
                    }
                }
            } else {
                println!(
                    "Scaffolded QuestForge project at {} ({} files)",
                    report.target_dir.display(),
                    report.created_files.len()
                );
            }
            if !args.skip_doctor && !args.json {
                println!("\nRunning post-init environment check (`quest doctor`)...");
                let _ = run_doctor(DoctorArgs {
                    project_dir: report.target_dir,
                    ci: false,
                    strict: false,
                    fix: false,
                    json: false,
                });
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("quest init failed: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run_sync(args: SyncArgs) -> ExitCode {
    match ProjectScaffolder::sync_project(&args.project_dir) {
        Ok(report) => {
            if args.json {
                match serde_json::to_string_pretty(&report) {
                    Ok(json) => println!("{json}"),
                    Err(err) => {
                        eprintln!("failed to serialize sync report: {err}");
                        return ExitCode::FAILURE;
                    }
                }
            } else {
                println!(
                    "Synchronized {} file(s) in {} ({:?}):",
                    report.synced_files.len(),
                    report.project_dir.display(),
                    report.engine
                );
                for f in &report.synced_files {
                    println!("  - {}", f.display());
                }
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("quest sync failed: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run_check(args: CheckArgs) -> ExitCode {
    match ProjectScaffolder::check_project(&args.project_dir) {
        Ok(report) => {
            if args.json {
                match serde_json::to_string_pretty(&report) {
                    Ok(json) => println!("{json}"),
                    Err(err) => {
                        eprintln!("failed to serialize check report: {err}");
                        return ExitCode::FAILURE;
                    }
                }
            } else {
                for finding in &report.findings {
                    let badge = if finding.passed { "PASS" } else { "FAIL" };
                    println!(
                        "[{badge}] {} ({}): {}",
                        finding.code, finding.name, finding.message
                    );
                    if let Some(fix) = &finding.remediation {
                        println!("       fix: {fix}");
                    }
                }
            }
            if report.passed {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(err) => {
            eprintln!("quest check failed: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run_ci(args: CiArgs) -> ExitCode {
    if let Some(provider) = args.emit_pipeline {
        print!(
            "{}",
            CiOrchestrator::render_pipeline_template(provider.into())
        );
        return ExitCode::SUCCESS;
    }

    let opts = CiOptions {
        project_dir: args.project_dir,
        output_dir: args.output_dir,
        release_version: args.release_version,
        upload_channel: args.upload_channel,
        app_id: args.app_id,
        dry_run: args.dry_run,
    };

    match CiOrchestrator::run(&opts) {
        Ok(report) => {
            if args.json {
                match serde_json::to_string_pretty(&report) {
                    Ok(json) => println!("{json}"),
                    Err(err) => {
                        eprintln!("failed to serialize ci report: {err}");
                        return ExitCode::FAILURE;
                    }
                }
            } else {
                println!(
                    "QuestForge CI {} ({})",
                    if args.dry_run { "planned" } else { "succeeded" },
                    report.metadata.apk_path.display()
                );
                println!("  - metadata: {}", report.metadata_path.display());
                println!("  - sbom:     {}", report.sbom_path.display());
                println!(
                    "  - findings: {} advisory notice(s)",
                    report.vulnerabilities.len()
                );
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("quest ci failed: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run_verify(args: VerifyArgs) -> ExitCode {
    let opts = VerifyOptions {
        apk_path: args.apk,
        project_dir: args.project_dir,
        max_apk_bytes: None,
    };

    match ApkVerifier::verify(&opts) {
        Ok(report) => {
            let fmt: ReportFormat = args.format.into();
            match report.render_bytes(fmt) {
                Ok(bytes) => {
                    if let Some(out_path) = args.out {
                        if let Err(err) = std::fs::write(&out_path, &bytes) {
                            eprintln!("failed to write report '{}': {err}", out_path.display());
                            return ExitCode::FAILURE;
                        }
                    } else {
                        print!("{}", String::from_utf8_lossy(&bytes));
                    }
                }
                Err(err) => {
                    eprintln!("failed to render report: {err}");
                    return ExitCode::FAILURE;
                }
            }
            if report.has_failures() {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(err) => {
            eprintln!("quest verify failed: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run_mcp(args: McpArgs) -> ExitCode {
    if args.list_tools {
        match serde_json::to_string_pretty(&QuestMcpServer::tool_definitions()) {
            Ok(json) => {
                println!("{json}");
                return ExitCode::SUCCESS;
            }
            Err(err) => {
                eprintln!("failed to serialize MCP tools: {err}");
                return ExitCode::FAILURE;
            }
        }
    }

    let server = QuestMcpServer::new(args.workspace_root);
    match server.serve_stdio() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("quest mcp failed: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run_agent(args: AgentArgs) -> ExitCode {
    let Ok(quest_bin) = std::env::current_exe() else {
        eprintln!("quest agent failed: unable to locate current quest executable");
        return ExitCode::FAILURE;
    };

    if let Some(prompt) = args.prompt {
        let explicit_key = std::env::var(&args.api_key_env)
            .ok()
            .filter(|k| !k.trim().is_empty());
        let explicit_provider = if let Some(ref name) = args.provider {
            let Some(p) = AutoLlmCredentialResolver::provider_for_id(name) else {
                eprintln!(
                    "quest agent failed: unknown --provider '{name}' (expected: gemini, openai, anthropic)"
                );
                return ExitCode::FAILURE;
            };
            Some(p)
        } else {
            None
        };

        let role: AgentRole = args.role.into();
        let resolved = if explicit_key.is_some() {
            None
        } else if let Some(p) = explicit_provider {
            match p.resolve_credentials() {
                Some(cfg) => Some((p, cfg)),
                None if args.auth => {
                    match AutoLlmCredentialResolver::authenticate_provider(p, false) {
                        Ok(cfg) => Some((p, cfg)),
                        Err(err) => {
                            eprintln!(
                                "OAuth login failed ({err}); falling back to API key prompt."
                            );
                            prompt_for_provider_api_key(p.provider_id()).map(|key| {
                                (
                                    p,
                                    AutoLlmCredentialResolver::config_with_api_key(
                                        p.provider_id(),
                                        key,
                                    ),
                                )
                            })
                        }
                    }
                }
                None => None,
            }
        } else {
            match AutoLlmCredentialResolver::resolve_first_with_provider() {
                Some(found) => Some(found),
                None if args.auth => {
                    let Some(default_p) = AutoLlmCredentialResolver::provider_for_id("gemini")
                    else {
                        return ExitCode::FAILURE;
                    };
                    match AutoLlmCredentialResolver::authenticate_provider(default_p, false) {
                        Ok(cfg) => Some((default_p, cfg)),
                        Err(err) => {
                            eprintln!(
                                "OAuth login failed ({err}); falling back to API key prompt."
                            );
                            prompt_for_provider_api_key(default_p.provider_id()).map(|key| {
                                (
                                    default_p,
                                    AutoLlmCredentialResolver::config_with_api_key(
                                        default_p.provider_id(),
                                        key,
                                    ),
                                )
                            })
                        }
                    }
                }
                None => None,
            }
        };

        let api_key = explicit_key
            .or_else(|| resolved.as_ref().map(|(_, r)| r.api_key.clone()))
            .unwrap_or_default();
        let llm_url = args
            .llm_url
            .or_else(|| {
                std::env::var("QUESTFORGE_LLM_URL")
                    .ok()
                    .filter(|v| !v.trim().is_empty())
            })
            .or_else(|| resolved.as_ref().map(|(_, r)| r.endpoint_url.clone()))
            .unwrap_or_else(|| "https://api.openai.com/v1/chat/completions".to_string());
        let model = args
            .model
            .or_else(|| {
                std::env::var("QUESTFORGE_LLM_MODEL")
                    .ok()
                    .filter(|v| !v.trim().is_empty())
            })
            .or_else(|| resolved.as_ref().map(|(_, r)| r.default_model.clone()))
            .unwrap_or_else(|| "gpt-4o-mini".to_string());
        let provider = explicit_provider
            .or_else(|| {
                resolved
                    .as_ref()
                    .filter(|(_, r)| r.endpoint_url == llm_url)
                    .map(|(p, _)| *p)
            })
            .unwrap_or_else(|| AutoLlmCredentialResolver::provider_for_url(&llm_url));
        let auth_source = resolved
            .as_ref()
            .map(|(_, r)| r.auth_source.clone())
            .unwrap_or_else(|| {
                if api_key.starts_with("sk-ant-oat") {
                    "oauth".to_string()
                } else {
                    "api-key".to_string()
                }
            });
        let endpoint_cfg = LlmEndpointConfig {
            provider_id: provider.provider_id().to_string(),
            endpoint_url: llm_url,
            default_model: model,
            api_key,
            auth_source,
        };
        let mut llm_res = QuestXrAgent::run_llm_stdio_client_with_spi_and_role(
            &quest_bin,
            &args.project_dir,
            &prompt,
            args.max_turns,
            provider,
            &endpoint_cfg,
            role,
        );
        if let Err(ref err) = llm_res {
            if args.auth && AutoLlmCredentialResolver::is_auth_error(err) {
                eprintln!(
                    "OAuth authentication failed for '{}' ({err}); falling back to API key prompt.",
                    provider.provider_id()
                );
                if let Some(key) = prompt_for_provider_api_key(provider.provider_id()) {
                    let mut fallback_cfg =
                        AutoLlmCredentialResolver::config_with_api_key(provider.provider_id(), key);
                    fallback_cfg.endpoint_url = endpoint_cfg.endpoint_url.clone();
                    fallback_cfg.default_model = endpoint_cfg.default_model.clone();
                    llm_res = QuestXrAgent::run_llm_stdio_client_with_spi_and_role(
                        &quest_bin,
                        &args.project_dir,
                        &prompt,
                        args.max_turns,
                        provider,
                        &fallback_cfg,
                        role,
                    );
                }
            }
        }
        return match llm_res {
            Ok(report) => {
                if args.json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&report).unwrap_or_default()
                    );
                } else {
                    println!(
                        "QuestXrAgent LLM finished (model={}, tools={}, steps={}):",
                        report.model,
                        report.discovered_tools.len(),
                        report.steps.len()
                    );
                    for s in &report.steps {
                        let badge = if s.ok { "OK" } else { "ERR" };
                        println!("  {}. [{}] {} -> {}", s.step, badge, s.tool, s.summary);
                    }
                    println!("\n{}", report.final_answer);
                }
                ExitCode::SUCCESS
            }
            Err(err) => {
                eprintln!("quest agent (LLM) failed: {err}");
                ExitCode::FAILURE
            }
        };
    }

    let role: AgentRole = args.role.into();
    let goal = args
        .goal
        .map(Into::into)
        .unwrap_or_else(|| role.default_goal());
    let opts = AgentOptions {
        project_dir: args.project_dir,
        output_dir: args.output_dir,
        release_version: args.release_version,
        goal,
        dry_run: args.dry_run,
    };

    match QuestXrAgent::run_stdio_client(&quest_bin, &opts) {
        Ok(report) => {
            if args.json {
                match serde_json::to_string_pretty(&report) {
                    Ok(json) => println!("{json}"),
                    Err(err) => {
                        eprintln!("failed to serialize agent report: {err}");
                        return ExitCode::FAILURE;
                    }
                }
            } else {
                println!(
                    "QuestXrAgent finished (goal={:?}, passed={}, tools={}):",
                    report.goal,
                    report.passed,
                    report.discovered_tools.len()
                );
                for s in &report.steps {
                    let badge = if s.ok { "OK" } else { "ERR" };
                    println!("  {}. [{}] {} -> {}", s.step, badge, s.tool, s.summary);
                }
                if let Some(score) = report.verify_score {
                    println!("  Final VRC score: {score}/100");
                }
            }
            if report.passed {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(err) => {
            eprintln!("quest agent failed: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run_auth(args: AuthArgs) -> ExitCode {
    if args.status {
        for provider in AutoLlmCredentialResolver::built_in_providers() {
            let id = provider.provider_id();
            match provider.resolve_credentials() {
                Some(cfg) => println!(
                    "{id}: ready (source={}, model={}, url={})",
                    cfg.auth_source, cfg.default_model, cfg.endpoint_url
                ),
                None => println!("{id}: missing (run `quest auth --provider {id}`)"),
            }
        }
        return ExitCode::SUCCESS;
    }

    let Some(provider) = AutoLlmCredentialResolver::provider_for_id(&args.provider) else {
        eprintln!(
            "quest auth failed: unknown --provider '{}' (expected: gemini, openai, anthropic)",
            args.provider
        );
        return ExitCode::FAILURE;
    };

    match AutoLlmCredentialResolver::authenticate_provider(provider, args.force) {
        Ok(cfg) => {
            println!(
                "Authenticated provider '{}' (source={}, model={})",
                cfg.provider_id, cfg.auth_source, cfg.default_model
            );
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("OAuth login failed for '{}': {err}", provider.provider_id());
            if let Some(key) = prompt_for_provider_api_key(provider.provider_id()) {
                let cfg =
                    AutoLlmCredentialResolver::config_with_api_key(provider.provider_id(), key);
                println!(
                    "Configured provider '{}' via prompted API key (source={}, model={}; set {} to persist)",
                    cfg.provider_id,
                    cfg.auth_source,
                    cfg.default_model,
                    AutoLlmCredentialResolver::provider_api_key_env(provider.provider_id())
                );
                ExitCode::SUCCESS
            } else {
                eprintln!("quest auth failed: {err}");
                ExitCode::FAILURE
            }
        }
    }
}

fn prompt_for_provider_api_key(provider_id: &str) -> Option<String> {
    if !io::stdin().is_terminal() {
        return None;
    }
    let env_name = AutoLlmCredentialResolver::provider_api_key_env(provider_id);
    eprint!("Enter {provider_id} API key ({env_name}, leave blank to cancel): ");
    let _ = io::stderr().flush();
    let mut line = String::new();
    io::stdin().read_line(&mut line).ok()?;
    let trimmed = line.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn run_ops(args: OpsArgs) -> ExitCode {
    let mut inv = match FleetInventory::load(&args.registry) {
        Ok(i) => i,
        Err(err) => {
            eprintln!("quest ops failed to load registry: {err}");
            return ExitCode::FAILURE;
        }
    };
    let env = OpsEnv::System;

    match args.action {
        OpsAction::Discover { group } => match FleetManager::discover(&mut inv, &group, &env) {
            Ok(devs) => {
                if let Err(err) = inv.save(&args.registry) {
                    eprintln!("failed to save registry: {err}");
                    return ExitCode::FAILURE;
                }
                if args.json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&devs).unwrap_or_default()
                    );
                } else {
                    println!("Discovered {} device(s):", devs.len());
                    for d in devs {
                        println!("  - {} [{}] (group: {})", d.serial, d.state, d.group);
                    }
                }
                ExitCode::SUCCESS
            }
            Err(err) => {
                eprintln!("quest ops discover failed: {err}");
                ExitCode::FAILURE
            }
        },
        OpsAction::Register { serial, group } => {
            let rec = FleetManager::register_device(&mut inv, &serial, &group);
            if let Err(err) = inv.save(&args.registry) {
                eprintln!("failed to save registry: {err}");
                return ExitCode::FAILURE;
            }
            if args.json {
                println!("{}", serde_json::to_string_pretty(&rec).unwrap_or_default());
            } else {
                println!("Registered {} in group '{}'", rec.serial, rec.group);
            }
            ExitCode::SUCCESS
        }
        OpsAction::List => {
            if args.json {
                println!("{}", serde_json::to_string_pretty(&inv).unwrap_or_default());
            } else {
                println!("Registered devices ({}):", inv.devices.len());
                for d in inv.devices.values() {
                    println!(
                        "  - {} [{}] group={} fw={:?} battery={:?}%",
                        d.serial, d.state, d.group, d.firmware_version, d.battery_level
                    );
                }
            }
            ExitCode::SUCCESS
        }
        OpsAction::Telemetry { serial } => {
            match FleetManager::refresh_telemetry(&mut inv, &serial, &env) {
                Ok(rec) => {
                    if let Err(err) = inv.save(&args.registry) {
                        eprintln!("failed to save registry: {err}");
                        return ExitCode::FAILURE;
                    }
                    if args.json {
                        println!("{}", serde_json::to_string_pretty(&rec).unwrap_or_default());
                    } else {
                        println!(
                            "{}: fw={:?}, battery={:?}% ({:?})",
                            rec.serial, rec.firmware_version, rec.battery_level, rec.battery_health
                        );
                    }
                    ExitCode::SUCCESS
                }
                Err(err) => {
                    eprintln!("quest ops telemetry failed: {err}");
                    ExitCode::FAILURE
                }
            }
        }
        OpsAction::Deploy {
            target,
            apk,
            schedule,
        } => match FleetManager::deploy_apk(&mut inv, &target, &apk, schedule.as_deref(), &env) {
            Ok(rec) => {
                if let Err(err) = inv.save(&args.registry) {
                    eprintln!("failed to save registry: {err}");
                    return ExitCode::FAILURE;
                }
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&rec).unwrap_or_default());
                } else {
                    println!(
                        "Deployment {}: {} -> {} ({} device(s))",
                        rec.status,
                        rec.apk_path.display(),
                        rec.target,
                        rec.devices.len()
                    );
                }
                ExitCode::SUCCESS
            }
            Err(err) => {
                eprintln!("quest ops deploy failed: {err}");
                ExitCode::FAILURE
            }
        },
        OpsAction::Rollback { target } => match FleetManager::rollback(&mut inv, &target, &env) {
            Ok(rec) => {
                if let Err(err) = inv.save(&args.registry) {
                    eprintln!("failed to save registry: {err}");
                    return ExitCode::FAILURE;
                }
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&rec).unwrap_or_default());
                } else {
                    println!("Rolled back {} to {}", rec.target, rec.apk_path.display());
                }
                ExitCode::SUCCESS
            }
            Err(err) => {
                eprintln!("quest ops rollback failed: {err}");
                ExitCode::FAILURE
            }
        },
        OpsAction::Logcat { serial, out } => {
            match FleetManager::capture_logcat(&serial, &out, &env) {
                Ok(path) => {
                    println!("Saved logcat to {}", path.display());
                    ExitCode::SUCCESS
                }
                Err(err) => {
                    eprintln!("quest ops logcat failed: {err}");
                    ExitCode::FAILURE
                }
            }
        }
        OpsAction::Screenshot { serial, out } => {
            match FleetManager::capture_screenshot(&serial, &out, &env) {
                Ok(path) => {
                    println!("Saved screenshot to {}", path.display());
                    ExitCode::SUCCESS
                }
                Err(err) => {
                    eprintln!("quest ops screenshot failed: {err}");
                    ExitCode::FAILURE
                }
            }
        }
        OpsAction::Agent { server, group } => {
            match FleetManager::poll_server_once(&mut inv, &server, &group, &env) {
                Ok(report) => {
                    if let Err(err) = inv.save(&args.registry) {
                        eprintln!("failed to save registry: {err}");
                        return ExitCode::FAILURE;
                    }
                    if args.json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&report).unwrap_or_default()
                        );
                    } else {
                        println!(
                            "Agent sync complete: {} device(s) discovered, {} deployment(s) executed",
                            report.discovered,
                            report.executed_deployments.len()
                        );
                    }
                    ExitCode::SUCCESS
                }
                Err(err) => {
                    eprintln!("quest ops agent failed: {err}");
                    ExitCode::FAILURE
                }
            }
        }
        OpsAction::HmsPush {
            project_dir,
            provider,
            app_id,
            apk,
            channel,
            group,
            endpoint,
            token_env,
        } => {
            let mdm_cfg = project_dir
                .as_deref()
                .and_then(|dir| QuestforgeConfig::load_from_dir(dir).ok())
                .and_then(|cfg| cfg.mdm);

            let resolved_provider =
                provider.map(MdmProvider::from).unwrap_or_else(|| {
                    match mdm_cfg.as_ref().map(|m| m.provider) {
                        Some(MdmTarget::ManageXr) => MdmProvider::ManageXr,
                        Some(MdmTarget::ArborXr) => MdmProvider::ArborXr,
                        _ => MdmProvider::MetaHms,
                    }
                });
            let resolved_app_id = app_id
                .or_else(|| mdm_cfg.as_ref().map(|m| m.app_id.clone()))
                .unwrap_or_default();
            if resolved_app_id.trim().is_empty() {
                eprintln!("quest ops hms-push failed: provide --app-id or configure mdm.app_id in quest_forge.yaml");
                return ExitCode::FAILURE;
            }
            let resolved_channel = channel
                .or_else(|| mdm_cfg.as_ref().map(|m| m.channel.clone()))
                .unwrap_or_else(|| "PRODUCTION".to_string());
            let resolved_group = group
                .or_else(|| mdm_cfg.as_ref().map(|m| m.device_group.clone()))
                .unwrap_or_else(|| "default".to_string());
            let resolved_endpoint =
                endpoint.or_else(|| mdm_cfg.as_ref().and_then(|m| m.endpoint.clone()));
            let resolved_token_env = token_env
                .or_else(|| mdm_cfg.as_ref().map(|m| m.token_env.clone()))
                .unwrap_or_else(|| "QUESTFORGE_MDM_TOKEN".to_string());

            let token = std::env::var(&resolved_token_env).unwrap_or_default();
            let req = HmsReleaseRequest {
                provider: resolved_provider,
                app_id: resolved_app_id,
                apk_path: apk,
                channel: resolved_channel,
                device_group: resolved_group,
                endpoint: resolved_endpoint,
                access_token: token,
            };
            match FleetManager::publish_to_mdm(&req, &env) {
                Ok(receipt) => {
                    if args.json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&receipt).unwrap_or_default()
                        );
                    } else {
                        println!(
                            "Published to {} (release: {}, status: {}, group: {}, sha256: {})",
                            receipt.provider,
                            receipt.release_id,
                            receipt.status,
                            receipt.device_group,
                            receipt.apk_sha256
                        );
                    }
                    ExitCode::SUCCESS
                }
                Err(err) => {
                    eprintln!("quest ops hms-push failed: {err}");
                    ExitCode::FAILURE
                }
            }
        }
        OpsAction::StoreUpload {
            app_id,
            apk,
            channel,
            app_secret_env,
            notes,
            ovr_bin,
        } => {
            let app_secret = std::env::var(&app_secret_env).unwrap_or_default();
            let req = StoreUploadRequest {
                app_id,
                app_secret,
                apk_path: apk,
                channel,
                release_notes: notes,
                ovr_bin,
            };
            match FleetManager::upload_to_store(&req, &env) {
                Ok(receipt) => {
                    if args.json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&receipt).unwrap_or_default()
                        );
                    } else {
                        println!(
                            "Uploaded to Meta Horizon Store (app_id: {}, channel: {}, sha256: {})",
                            receipt.app_id, receipt.channel, receipt.apk_sha256
                        );
                    }
                    ExitCode::SUCCESS
                }
                Err(err) => {
                    eprintln!("quest ops store-upload failed: {err}");
                    ExitCode::FAILURE
                }
            }
        }
        OpsAction::Store {
            action,
            app_id,
            apk,
            channel,
            build_id,
            dir,
            sku,
            notes,
            app_secret_env,
            ovr_bin,
        } => {
            let app_secret = std::env::var(&app_secret_env).unwrap_or_default();
            let req = OvrStoreRequest {
                action: action.into(),
                app_id,
                app_secret,
                apk_path: apk,
                channel,
                build_id,
                dir_path: dir,
                dlc_sku: sku,
                release_notes: notes,
                ovr_bin,
            };
            match FleetManager::execute_ovr_store(&req, &env) {
                Ok(receipt) => {
                    if args.json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&receipt).unwrap_or_default()
                        );
                    } else {
                        println!(
                            "Meta Store {} succeeded (app_id: {}, channel: {}, build_id: {})",
                            receipt.action.as_subcommand(),
                            receipt.app_id,
                            receipt.channel.as_deref().unwrap_or("-"),
                            receipt.build_id.as_deref().unwrap_or("-")
                        );
                    }
                    ExitCode::SUCCESS
                }
                Err(err) => {
                    eprintln!("quest ops store failed: {err}");
                    ExitCode::FAILURE
                }
            }
        }
    }
}

fn run_serve(args: ServeArgs) -> ExitCode {
    let db_url = args
        .database_url
        .or_else(|| std::env::var("QUESTFORGE_DATABASE_URL").ok());
    let store = match db_url {
        Some(url) => match PostgresStore::connect(&url) {
            Ok(pg) => StoreBackend::Postgres(std::sync::Arc::new(pg)),
            Err(err) => {
                eprintln!("quest serve failed to connect to PostgreSQL: {err}");
                return ExitCode::FAILURE;
            }
        },
        None => StoreBackend::default(),
    };
    let jwt_secret = args
        .jwt_secret
        .or_else(|| std::env::var("QUESTFORGE_JWT_SECRET").ok());
    let auth = match jwt_secret {
        Some(secret) => AuthMode::Jwt(JwtVerifier::new(secret.into_bytes())),
        None => AuthMode::DevHeader,
    };
    let server = ControlPlaneServer::with_backend(store, auth, args.web_root);
    println!("QuestForge control plane listening on http://{}", args.addr);
    match server.serve(&args.addr) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("quest serve failed: {err}");
            ExitCode::FAILURE
        }
    }
}
