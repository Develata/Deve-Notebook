use super::super::{
    BACKUP_RESTORE_MAX_ENCRYPTED_BYTES, BACKUP_RESTORE_MAX_PACKS,
    BACKUP_RESTORE_MAX_PLAINTEXT_BYTES, BackupRestoreError, BackupRestoreResourceBudgetInput,
    admit_restore_candidate, validate_backup_restore_resource_budget,
};
use super::{input, numbered_digest};

#[test]
fn backup_restore_candidate_rejects_resource_budget_excess() {
    let mut candidate_input = input();
    candidate_input.pack_count = BACKUP_RESTORE_MAX_PACKS + 1;
    candidate_input.pack_digests = (1..=candidate_input.pack_count)
        .map(numbered_digest)
        .collect();
    let err = admit_restore_candidate(candidate_input)
        .expect_err("restore candidate pack count budget must fail closed");
    assert_eq!(err, BackupRestoreError::PackCountBudgetExceeded);

    let err = validate_backup_restore_resource_budget(BackupRestoreResourceBudgetInput {
        pack_count: 1,
        encrypted_bytes: BACKUP_RESTORE_MAX_ENCRYPTED_BYTES + 1,
        plaintext_bytes: 0,
    })
    .expect_err("encrypted bytes budget must fail closed");
    assert_eq!(err, BackupRestoreError::EncryptedBytesBudgetExceeded);

    let err = validate_backup_restore_resource_budget(BackupRestoreResourceBudgetInput {
        pack_count: 1,
        encrypted_bytes: 0,
        plaintext_bytes: BACKUP_RESTORE_MAX_PLAINTEXT_BYTES + 1,
    })
    .expect_err("plaintext bytes budget must fail closed");
    assert_eq!(err, BackupRestoreError::PlaintextBytesBudgetExceeded);
}
