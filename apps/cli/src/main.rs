//! plan_ref:
//!   - 14_commands#cli-commands
//!
//! `deve_cli` sidecar binary entrypoint. The command surface is implemented in
//! the crate-level CLI runner so release aliases share one dispatch path.

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
