//! plan_ref: infra

use anyhow::{Result, bail};
use std::env;
use std::process::ExitCode;

mod context;
mod dev_runbook;
mod diff_color;
mod graph;
mod i18n_formatting;
mod i18n_hardcoded;
mod network;
mod release;
mod rendering;
mod search;
mod spec;
mod storage_repo;
mod ui_focus;
mod ui_token;
mod ui_z_index;

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
        "dev-runbook" => dev_runbook::run(),
        "diff-color" => diff_color::run(),
        "graph" => graph::run(),
        "i18n-formatting" => i18n_formatting::run(),
        "i18n-hardcoded" => i18n_hardcoded::run(),
        "rendering" => rendering::run(),
        "search" => search::run(),
        "ui-token" => ui_token::run(),
        "ui-z-index" => ui_z_index::run(),
        "ui-focus" => ui_focus::run(),
        "all" => {
            storage_repo::run()?;
            network::run()?;
            release::run()?;
            dev_runbook::run()?;
            diff_color::run()?;
            graph::run()?;
            i18n_formatting::run()?;
            i18n_hardcoded::run()?;
            rendering::run()?;
            search::run()?;
            ui_token::run()?;
            ui_z_index::run()?;
            ui_focus::run()
        }
        "-h" | "--help" | "help" => {
            println!(
                "Usage: deve_baseline <storage-repo|network|release|dev-runbook|diff-color|graph|i18n-formatting|i18n-hardcoded|rendering|search|ui-token|ui-z-index|ui-focus|all>"
            );
            Ok(())
        }
        other => {
            bail!(
                "unknown baseline check '{other}'. expected one of: storage-repo, network, release, dev-runbook, diff-color, graph, i18n-formatting, i18n-hardcoded, rendering, search, ui-token, ui-z-index, ui-focus, all"
            )
        }
    }
}
