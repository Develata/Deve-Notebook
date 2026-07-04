//! plan_ref:
//!   - 14_commands#cli-commands
//!
//! Git mirror CLI command rendering helpers.

pub(super) fn mirror_command(repo_name: &str, retry_out_of_sync: bool) -> String {
    let retry = if retry_out_of_sync {
        " --retry-out-of-sync"
    } else {
        ""
    };
    format!(
        "deve_cli ngit mirror --repo {}{}",
        shell_quote(repo_name),
        retry
    )
}

pub(super) fn ngit_command(action: &str, repo_name: &str, retry_out_of_sync: bool) -> String {
    let retry = if retry_out_of_sync {
        " --retry-out-of-sync"
    } else {
        ""
    };
    format!(
        "deve_cli ngit {} --repo {}{}",
        action,
        shell_quote(repo_name),
        retry
    )
}

pub(super) fn shell_quote(value: &str) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/' | ':'))
    {
        return value.to_string();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}
