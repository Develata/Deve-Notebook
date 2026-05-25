//! plan_ref:
//!   - 03_storage/index#git-ecosystem-coexistence
//!   - 05_diff_logic#git-mirror-lifecycle
//!   - 14_commands#cli-commands
//!
//! Human-readable Git mirror push diagnostics.

use deve_core::git_bridge::GitMirrorPushReport;

use super::command::{git_command, shell_quote};

pub(crate) fn print_push_report(repo_name: &str, report: &GitMirrorPushReport) {
    for line in push_report_lines(repo_name, report) {
        println!("{line}");
    }
}

fn push_report_lines(repo_name: &str, report: &GitMirrorPushReport) -> Vec<String> {
    let mut lines = vec![format!(
        "git_push[{repo_name}]: pushed={} remote={} branch={} head={} blockers={}",
        report.pushed,
        report.remote.as_deref().unwrap_or("-"),
        report.branch.as_deref().unwrap_or("-"),
        report.head.as_deref().unwrap_or("-"),
        report.blockers.len()
    )];
    if let Some(url) = &report.remote_url {
        lines.push(format!("  remote_url: {url}"));
    }
    for (index, blocker) in report.blockers.iter().enumerate() {
        lines.push(format!(
            "  blocker[{}]: location={} reason={}",
            index + 1,
            blocker.location,
            blocker.reason
        ));
        lines.push(format!(
            "    hint: {}",
            push_blocker_hint(repo_name, &blocker.location)
        ));
    }
    if report.pushed {
        lines.push(
            "  push_hint: Git mirror HEAD was pushed; Deve ledger remains authority".to_string(),
        );
    } else if report.blockers.is_empty() {
        lines.push("  push_hint: no remote push was needed".to_string());
    } else {
        lines.push(
            "  push_hint: no remote push was performed; fix the blocker hint(s) above first"
                .to_string(),
        );
    }
    lines
}

fn push_blocker_hint(repo_name: &str, location: &str) -> String {
    match location {
        "mirror_not_ready" => format!(
            "run `{}` and ensure `.git` exists with `.gitignore` protecting `.notegit/`",
            git_command("status", repo_name, false)
        ),
        "deve_source_control" => {
            "stage/commit/discard current Deve Source Control changes before pushing".to_string()
        }
        "git_worktree" => format!(
            "clean Git worktree or run `{}` before pushing",
            import_apply_command(repo_name)
        ),
        "git_history_mapping" => format!(
            "run `{}` or `{}` so Git HEAD maps to latest Deve commit",
            git_command("export", repo_name, false),
            git_command("export", repo_name, true)
        ),
        "git_remote" => {
            "configure branch upstream/origin, or pass `--remote <remote> --branch <branch>`"
                .to_string()
        }
        "git_command" => {
            "check remote credentials/network and retry with explicit remote/branch".to_string()
        }
        _ => "inspect the blocker reason and rerun after repair".to_string(),
    }
}

fn import_apply_command(repo_name: &str) -> String {
    format!(
        "deve_cli git import --apply --repo {}",
        shell_quote(repo_name)
    )
}

#[cfg(test)]
mod tests;
