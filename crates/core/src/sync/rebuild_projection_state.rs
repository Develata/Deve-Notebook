//! plan_ref:
//!   - 03_storage/projection#projection-contract
//!   - 04_repository#tree-projection-contract

use crate::ledger::RepoManager;
use crate::ledger::manager::structure_projection;
use crate::ledger::schema::{
    DOCID_TO_PATH, INODE_TO_DOCID, INODE_TO_NODEID, LEDGER_OPS, NODEID_TO_META, PATH_TO_DOCID,
    PATH_TO_NODEID,
};
use crate::models::{LedgerEvent, StructureOp, deserialize_ledger_entry};
use anyhow::Result;
use redb::ReadableTable;

/// Invariants:
/// - local rebuild 必须先清空 projection tables，再由 Structure Facts 重新折叠。
/// - rebuild 不得依赖旧 path/doc/node projection 残影。
pub(super) fn rebuild_local_projection_state(repo: &RepoManager, repo_name: &str) -> Result<()> {
    repo.run_on_local_repo(repo_name, |db| {
        let read = db.begin_read()?;
        let ledger = read.open_table(LEDGER_OPS)?;
        let mut ops = Vec::<StructureOp>::new();
        for item in ledger.iter()? {
            let (_, bytes) = item?;
            let entry = deserialize_ledger_entry(bytes.value())?;
            if let LedgerEvent::Structure(op) = entry.event {
                ops.push(op);
            }
        }
        drop(ledger);
        drop(read);

        reset_local_projection_tables(db)?;
        for op in &ops {
            structure_projection::apply(db, op)?;
        }
        Ok(())
    })
}

fn reset_local_projection_tables(db: &redb::Database) -> Result<()> {
    let write = db.begin_write()?;
    let _ = write.delete_table(PATH_TO_DOCID)?;
    let _ = write.delete_table(DOCID_TO_PATH)?;
    let _ = write.delete_table(INODE_TO_DOCID)?;
    let _ = write.delete_table(NODEID_TO_META)?;
    let _ = write.delete_table(PATH_TO_NODEID)?;
    let _ = write.delete_table(INODE_TO_NODEID)?;
    let _ = write.open_table(PATH_TO_DOCID)?;
    let _ = write.open_table(DOCID_TO_PATH)?;
    let _ = write.open_table(INODE_TO_DOCID)?;
    let _ = write.open_table(NODEID_TO_META)?;
    let _ = write.open_table(PATH_TO_NODEID)?;
    let _ = write.open_table(INODE_TO_NODEID)?;
    write.commit()?;
    Ok(())
}
