//! plan_ref:
//!   - 03_storage/authority#local-authority-owner-contract

use super::super::*;
use super::{RepoAuthoritySlotSnapshot, new_runtime};
use crate::ledger::schema::LEDGER_OPS;

#[test]
fn existing_admission_preserves_failure_while_sealing_repair_state() -> anyhow::Result<()> {
    let (dir, runtime, repo_id) = new_runtime()?;
    let lease = runtime.lease(repo_id)?;
    let write = lease.db().begin_write()?;
    write.delete_table(LEDGER_OPS)?;
    write.commit()?;
    drop(lease);
    drop(runtime);

    let reopened = LocalAuthorityRuntime::empty(dir.path());
    let error = match reopened.admit_existing(repo_id) {
        Ok(_) => panic!("incomplete authority tables must fail closed"),
        Err(error) => error,
    };

    assert!(
        error
            .to_string()
            .contains("ledger_ops authority table missing"),
        "the caller must retain the exact admission diagnosis: {error}"
    );
    assert_eq!(
        reopened.snapshot_for_test(repo_id)?,
        Some(RepoAuthoritySlotSnapshot::RepairRequired { generation: 1 })
    );
    Ok(())
}
