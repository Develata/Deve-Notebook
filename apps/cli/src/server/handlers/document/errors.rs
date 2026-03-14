use crate::server::channel::DualChannel;
use deve_core::protocol::{ServerError, ServerErrorCode};

fn error_code(err: &anyhow::Error) -> ServerErrorCode {
    let detail = err.to_string();
    let lower = detail.to_ascii_lowercase();
    if detail.starts_with("Document not found:") || detail.starts_with("Repository not found:") {
        return ServerErrorCode::StorageNotFound;
    }
    if lower.contains("database already open")
        || lower.contains("cannot acquire lock")
        || lower.contains("db locked")
        || lower.contains("database is locked")
    {
        return ServerErrorCode::StorageDbLocked;
    }
    if lower.contains("local repo operation requested on remote branch")
        || lower.contains("repo selector mismatch")
        || lower.contains("remote session lost repo name")
    {
        return ServerErrorCode::ScRepoContextInvalid;
    }
    ServerErrorCode::RequestFailed
}

pub(crate) fn send_doc_error(ch: &DualChannel, context: &str, err: anyhow::Error) {
    ch.send_protocol_error(ServerError::with_detail(
        error_code(&err),
        format!("{}: {}", context, err),
    ));
}

#[cfg(test)]
mod tests {
    use super::error_code;
    use deve_core::protocol::ServerErrorCode;

    #[test]
    fn classifies_missing_docs_and_repos_as_storage_not_found() {
        assert_eq!(
            error_code(&anyhow::anyhow!("Document not found: abc")),
            ServerErrorCode::StorageNotFound
        );
        assert_eq!(
            error_code(&anyhow::anyhow!("Repository not found: wiki")),
            ServerErrorCode::StorageNotFound
        );
    }

    #[test]
    fn classifies_locked_databases_as_storage_db_locked() {
        assert_eq!(
            error_code(&anyhow::anyhow!("Database already open. Cannot acquire lock.")),
            ServerErrorCode::StorageDbLocked
        );
    }

    #[test]
    fn classifies_repo_scope_drift_as_repo_context_invalid() {
        assert_eq!(
            error_code(&anyhow::anyhow!(
                "Repo selector mismatch: repo_id resolved to default, repo_name resolved to test"
            )),
            ServerErrorCode::ScRepoContextInvalid
        );
    }
}
