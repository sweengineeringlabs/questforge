//! CLI execution coordination for `quest`.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand, ValueEnum};
use questforge_config::EngineKind;
use questforge_doctor::{CheckStatus, DoctorOptions, DoctorRunner};
use questforge_init::{InitOptions, ProjectScaffolder};
use questforge_toolchain::{SetupOptions, ToolchainProvisioner};

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
                    let status = if tool.cache_hit { "cached" } else { "installed" };
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
