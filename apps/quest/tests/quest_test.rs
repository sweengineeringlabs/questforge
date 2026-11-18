//! Integration tests for the `quest` CLI binary.

use std::process::Command;

#[test]
fn test_cli_help_exits_zero() {
    assert!(matches!(
        Command::new(env!("CARGO_BIN_EXE_quest")).arg("--help").status(),
        Ok(status) if status.success()
    ));
}
