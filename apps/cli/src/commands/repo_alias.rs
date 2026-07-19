//! plan_ref:
//!   - 04_repository#host-repo-alias-contract
//!   - 14_commands#repo-alias-command-contract
//!
//! Thin host CLI adapter for the core-owned repo alias runtime.

use anyhow::{Context, Result, anyhow};
use deve_core::ledger::{
    HOST_REPO_ALIAS_IMPORT_MAX_BYTES, HostRepoAliasImportSummary, HostRepoAliasRuntime,
};
use deve_core::models::RepoId;
use deve_core::utils::fs::{open_regular_file_read, sync_directory};
use std::io::{Read, Write};
use std::path::Path;

pub fn set(ledger_dir: &Path, repo_id: RepoId, alias: &str, expected_revision: u64) -> Result<()> {
    let runtime = HostRepoAliasRuntime::open_existing(ledger_dir)?;
    let result = runtime.set_alias(repo_id, alias, expected_revision)?;
    println!(
        "repo_alias_set: repo_id={} alias_revision={} changed={} alias={:?}",
        result.binding.repo_id, result.binding.alias_revision, result.changed, result.binding.alias
    );
    Ok(())
}

pub fn export(ledger_dir: &Path, output: &Path) -> Result<()> {
    let runtime = HostRepoAliasRuntime::open_existing(ledger_dir)?;
    let json = runtime.export_json()?;
    write_export(ledger_dir, output, json.as_bytes())?;
    println!(
        "repo_alias_export: output={:?} bytes={}",
        output,
        json.len()
    );
    Ok(())
}

pub fn import_aliases(ledger_dir: &Path, input: &Path, apply: bool) -> Result<()> {
    let bytes = read_bounded_input(input)?;
    let runtime = HostRepoAliasRuntime::open_existing(ledger_dir)?;
    let summary = if apply {
        runtime.apply_import_json(&bytes)?
    } else {
        runtime.preview_import_json(&bytes)?
    };
    print_summary(&summary, apply)?;
    Ok(())
}

fn read_bounded_input(path: &Path) -> Result<Vec<u8>> {
    let file = open_regular_file_read(path, "alias import file")
        .with_context(|| format!("failed to open alias import file {path:?}"))?;
    let metadata = file
        .metadata()
        .with_context(|| format!("failed to inspect alias import file {path:?}"))?;
    if metadata.len() > HOST_REPO_ALIAS_IMPORT_MAX_BYTES as u64 {
        return Err(anyhow!(
            "alias import exceeds file budget: actual={}, limit={}",
            metadata.len(),
            HOST_REPO_ALIAS_IMPORT_MAX_BYTES
        ));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take((HOST_REPO_ALIAS_IMPORT_MAX_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .with_context(|| format!("failed to read alias import file {path:?}"))?;
    if bytes.len() > HOST_REPO_ALIAS_IMPORT_MAX_BYTES {
        return Err(anyhow!(
            "alias import exceeded file budget while reading: actual>{}",
            HOST_REPO_ALIAS_IMPORT_MAX_BYTES
        ));
    }
    Ok(bytes)
}

fn write_export(ledger_dir: &Path, path: &Path, bytes: &[u8]) -> Result<()> {
    let requested_parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let metadata = std::fs::symlink_metadata(requested_parent)
        .with_context(|| format!("failed to inspect alias export parent {requested_parent:?}"))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() || is_reparse(&metadata) {
        return Err(anyhow!(
            "alias export parent is not a regular directory: {requested_parent:?}"
        ));
    }
    let parent = std::fs::canonicalize(requested_parent).with_context(|| {
        format!("failed to canonicalize alias export parent {requested_parent:?}")
    })?;
    let ledger = std::fs::canonicalize(ledger_dir)
        .with_context(|| format!("failed to canonicalize ledger directory {ledger_dir:?}"))?;
    if parent.starts_with(&ledger) {
        return Err(anyhow!(
            "alias export output must be outside the ledger authority tree: {path:?}"
        ));
    }
    let name = path
        .file_name()
        .filter(|name| !name.is_empty())
        .ok_or_else(|| anyhow!("alias export output has no file name: {path:?}"))?;
    let destination = parent.join(name);
    if std::fs::symlink_metadata(&destination).is_ok() {
        return Err(anyhow!(
            "refusing to overwrite existing alias export output: {destination:?}"
        ));
    }

    let temp = parent.join(format!(
        ".{}.{}.{}.tmp",
        name.to_string_lossy(),
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    let result = (|| -> Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .with_context(|| format!("failed to create alias export temp file {temp:?}"))?;
        file.write_all(bytes)?;
        file.sync_all()?;
        std::fs::hard_link(&temp, &destination).with_context(|| {
            format!(
                "failed to publish no-clobber alias export {destination:?}; the target may already exist"
            )
        })?;
        std::fs::remove_file(&temp)
            .with_context(|| format!("failed to remove alias export temp link {temp:?}"))?;
        sync_directory(&parent)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result
}

fn print_summary(summary: &HostRepoAliasImportSummary, apply: bool) -> Result<()> {
    let stdout = std::io::stdout();
    let stderr = std::io::stderr();
    write_summary(summary, apply, &mut stdout.lock(), &mut stderr.lock())
        .context("failed to write repo alias import summary")
}

fn write_summary(
    summary: &HostRepoAliasImportSummary,
    apply: bool,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> std::io::Result<()> {
    writeln!(
        stdout,
        "repo_alias_import: mode={} accepted={} changed={} unchanged={} skipped={}",
        if apply { "apply" } else { "dry-run" },
        summary.accepted,
        summary.changed,
        summary.unchanged,
        summary.skipped
    )?;
    for warning in &summary.warnings {
        let repo_id = warning
            .repo_id
            .map(|repo_id| repo_id.to_string())
            .unwrap_or_else(|| "<unparsed>".to_owned());
        writeln!(
            stderr,
            "warning: index={} repo_id={} reason={}",
            warning.index, repo_id, warning.reason
        )?;
    }
    Ok(())
}

#[cfg(windows)]
fn is_reparse(metadata: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn is_reparse(_metadata: &std::fs::Metadata) -> bool {
    false
}

#[cfg(test)]
mod tests;
