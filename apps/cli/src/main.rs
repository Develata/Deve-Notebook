//! plan_ref:
//!   - 14_commands#cli-commands
//!
//! `deve_cli` sidecar binary entrypoint. The command surface is implemented in
//! the crate-level CLI runner so release aliases share one dispatch path.

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    deve_cli::run_cli().await
}
