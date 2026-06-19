//! plan_ref: infra

use anyhow::{Result, bail};
use std::env;
use std::process::ExitCode;

mod context;
mod network;
mod release;
mod spec;
mod storage_repo;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let command = env::args().nth(1).unwrap_or_else(|| "--help".to_string());

    match command.as_str() {
        "storage-repo" => storage_repo::run(),
        "network" => network::run(),
        "release" => release::run(),
        "all" => {
            storage_repo::run()?;
            network::run()?;
            release::run()
        }
        "-h" | "--help" | "help" => {
            println!("Usage: deve_baseline <storage-repo|network|release|all>");
            Ok(())
        }
        other => {
            bail!(
                "unknown baseline check '{other}'. expected one of: storage-repo, network, release, all"
            )
        }
    }
}
