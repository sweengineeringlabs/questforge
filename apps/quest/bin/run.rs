//! CLI execution coordination for `quest`.

use std::process::ExitCode;

use clap::{Parser, Subcommand};

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
    Setup,
    /// Validate developer workstation or CI environment against quest_forge.yaml
    Doctor,
    /// Scaffold a Quest-ready Unity or Unreal repository
    Init,
}

/// Command-line runner for `quest`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Runner;

impl Runner {
    /// Executes the command-line interface.
    pub(crate) fn run() -> ExitCode {
        let cli = Cli::parse();
        match cli.command {
            Commands::Setup => {
                println!("quest setup: not yet implemented");
            }
            Commands::Doctor => {
                println!("quest doctor: not yet implemented");
            }
            Commands::Init => {
                println!("quest init: not yet implemented");
            }
        }
        ExitCode::SUCCESS
    }
}
