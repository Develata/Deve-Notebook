//! plan_ref:
//!   - 14_commands#cli-commands
//!
//! User-facing `deve` binary alias. It intentionally delegates to the same CLI
//! runner as `deve_cli` so aliases cannot diverge.

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    deve_cli::run_cli().await
}
