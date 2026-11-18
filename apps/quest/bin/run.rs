//! CLI execution coordination for `quest`.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand, ValueEnum};
use questforge_ci::{CiOptions, CiOrchestrator, CiProvider};
use questforge_config::EngineKind;
use questforge_doctor::{CheckStatus, DoctorOptions, DoctorRunner};
use questforge_init::{InitOptions, ProjectScaffolder};
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
    /// Build, package, generate SBOM, and publish Quest APK artifacts
    Ci(CiArgs),
    /// Verify APK release readiness, signatures, ABIs, and Meta VRC policy compliance
    Verify(VerifyArgs),
}

#[derive(Debug, Args)]
struct SetupArgs {
    /// Directory containing quest_forge.yaml (defaults to current directory)
    #[arg(long, default_value = ".")]
    project_dir: PathBuf,
    /// Override Android platform-tools (adb) version
    #[arg(long)]
    adb_version: Option<String>,
    /// Install Android cmdline-tools with the specified version
    #[arg(long)]
    cmdline_tools_version: Option<String>,
    /// Install ovr-platform-util with the specified version
    #[arg(long)]
    ovr_version: Option<String>,
    /// Download URL for ovr-platform-util
    #[arg(long)]
    ovr_url: Option<String>,
    /// Expected SHA-256 hex digest for ovr-platform-util
    #[arg(long)]
    ovr_sha256: Option<String>,
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
    /// Emit machine-readable JSON output
    #[arg(long)]
    json: bool,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CliEngine {
    Unity,
    Unreal,
}

impl From<CliEngine> for EngineKind {
    fn from(value: CliEngine) -> Self {
        match value {
            CliEngine::Unity => Self::Unity,
            CliEngine::Unreal => Self::Unreal,
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
            Commands::Ci(args) => run_ci(args),
            Commands::Verify(args) => run_verify(args),
        }
    }
}

fn run_setup(args: SetupArgs) -> ExitCode {
    let opts = SetupOptions {
        project_dir: Some(args.project_dir),
        adb_version: args.adb_version,
        cmdline_tools_version: args.cmdline_tools_version,
        ovr_version: args.ovr_version,
        ovr_url: args.ovr_url,
        ovr_sha256: args.ovr_sha256,
        cache_dir: args.cache_dir,
        include_cmdline_tools: false,
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
    let opts = DoctorOptions {
        project_dir: args.project_dir,
        ci: args.ci,
        strict: args.strict,
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
