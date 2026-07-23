//! plan_ref:
//!   - 14_commands#cli-commands
//!
//! User-facing `deve` binary alias. It intentionally delegates to the same CLI
//! runner as `deve_cli` so aliases cannot diverge.

#[tokio::main]
async fn main() -> std::process::ExitCode {
    match deve_cli::run_cli().await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error:#}");
            std::process::ExitCode::from(deve_cli::process_exit_code(&error))
        }
    }
}
