//! plan_ref: infra

use anyhow::{Result, bail};
use std::env;
use std::process::ExitCode;

mod ai;
mod auth;
mod cargo_test;
mod cli_settings;
mod context;
mod dev_data_health;
mod dev_runbook;
mod diff_color;
mod foundation;
mod graph;
mod i18n_formatting;
mod i18n_hardcoded;
mod large_doc;
mod mobile;
mod network;
mod release;
mod rendering;
mod repo_file_ops;
mod search;
mod settings_local_feedback;
mod source_control;
mod spec;
mod storage_repo;
mod ui_dashboard_refresh;
mod ui_desktop;
mod ui_disconnect;
mod ui_focus;
mod ui_spa_routing;
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
        "auth" => auth::run(),
        "ai" => ai::run(),
        "cli-settings" => cli_settings::run(),
        "dev-data-health" => dev_data_health::run(),
        "foundation" => foundation::run(),
        "large-doc" => large_doc::run(),
        "mobile" => mobile::run(),
        "repo-file-ops" => repo_file_ops::run(),
        "settings-local-feedback" => settings_local_feedback::run(),
        "source-control" => source_control::run(),
        "ui-dashboard-refresh" => ui_dashboard_refresh::run(),
        "ui-desktop" => ui_desktop::run(),
        "ui-disconnect" => ui_disconnect::run(),
        "ui-spa-routing" => ui_spa_routing::run(),
        "all" => run_text_baselines(),
        "full" => run_full_baselines(),
        "-h" | "--help" | "help" => {
            println!(
                "Usage: deve_baseline <storage-repo|network|release|dev-runbook|diff-color|graph|i18n-formatting|i18n-hardcoded|rendering|search|ui-token|ui-z-index|ui-focus|auth|ai|cli-settings|dev-data-health|foundation|large-doc|mobile|repo-file-ops|settings-local-feedback|source-control|ui-dashboard-refresh|ui-desktop|ui-disconnect|ui-spa-routing|all|full>"
            );
            Ok(())
        }
        other => {
            bail!(
                "unknown baseline check '{other}'. run `deve_baseline --help` for the supported checks"
            )
        }
    }
}

fn run_text_baselines() -> Result<()> {
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
    ui_focus::run()?;
    auth::run_text()?;
    ai::run_text()?;
    cli_settings::run_text()?;
    dev_data_health::run_text()?;
    foundation::run_text()?;
    large_doc::run_text()?;
    mobile::run_text()?;
    settings_local_feedback::run_text()?;
    source_control::run_text()?;
    ui_dashboard_refresh::run_text()?;
    ui_desktop::run_text()?;
    ui_disconnect::run_text()?;
    ui_spa_routing::run_text()
}

fn run_full_baselines() -> Result<()> {
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
    ui_focus::run()?;
    auth::run()?;
    ai::run()?;
    cli_settings::run()?;
    dev_data_health::run()?;
    foundation::run()?;
    large_doc::run()?;
    mobile::run()?;
    repo_file_ops::run()?;
    settings_local_feedback::run()?;
    source_control::run()?;
    ui_dashboard_refresh::run()?;
    ui_desktop::run()?;
    ui_disconnect::run()?;
    ui_spa_routing::run()
}
