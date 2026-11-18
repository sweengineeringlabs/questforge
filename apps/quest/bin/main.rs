//! Command-line entry point for `quest`.

mod run;

fn main() -> std::process::ExitCode {
    run::Runner::run()
}
