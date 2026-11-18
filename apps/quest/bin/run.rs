//! CLI execution coordination for `quest`.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use questforge_doctor::{CheckStatus, DoctorOptions, DoctorRunner};
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
    Init,
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
            Commands::Init => {
                println!("quest init: not yet implemented");
                ExitCode::SUCCESS
            }
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
