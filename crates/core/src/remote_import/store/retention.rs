//! plan_ref:
//!   - 03_storage/authority#remote-import-workflow-tables
//!   - 06_backup#remote-import-state-machine

use super::decode_session;
use crate::models::RepoId;
use crate::remote_import::error::{RemoteImportError, RemoteImportResult};
use redb::ReadableTable;

pub(in crate::remote_import) const TERMINAL_RETENTION: usize = 64;

pub(super) fn prune_terminal_records(
    sessions: &mut redb::Table<'_, u128, &[u8]>,
    repo_id: RepoId,
) -> RemoteImportResult<()> {
    let mut eligible = sessions
        .iter()
        .map_err(RemoteImportError::storage)?
        .map(|row| {
            let (key, value) = row.map_err(RemoteImportError::storage)?;
            decode_session(key.value(), value.value(), repo_id)
        })
        .collect::<RemoteImportResult<Vec<_>>>()?
        .into_iter()
        .filter(|record| record.state.is_terminal() && !record.cleanup_pending)
        .collect::<Vec<_>>();
    eligible.sort_by_key(|record| std::cmp::Reverse(record.order));
    for record in eligible.into_iter().skip(TERMINAL_RETENTION) {
        sessions
            .remove(&record.session_id.as_u128())
            .map_err(RemoteImportError::storage)?;
    }
    Ok(())
}
