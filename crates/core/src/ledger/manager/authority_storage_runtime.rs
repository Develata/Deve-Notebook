//! plan_ref:
//!   - 03_storage/authority#facts-partition
//!   - 03_storage/projection#projection-contract
//!   - 10_rendering#document-authority-bridge

mod local_authority;

pub(crate) use local_authority::{
    BoundRepoAuthority, LocalAuthorityDiscovery, LocalAuthorityRuntime,
};
pub use local_authority::{LocalAuthorityError, PreparedRepoAuthority, RepoAuthorityLease};

use crate::ledger::manager::types::RepoManager;
use crate::ledger::ops;
use crate::models::{DocId, LedgerEntry, PeerId, RepoId};
use anyhow::Result;

pub(crate) struct AuthorityStorageRuntime<'a> {
    manager: &'a RepoManager,
}

impl<'a> AuthorityStorageRuntime<'a> {
    pub(crate) fn new(manager: &'a RepoManager) -> Self {
        Self { manager }
    }

    pub(crate) fn append_local_op(&self, entry: &LedgerEntry) -> Result<u64> {
        self.ensure_local_origin(&entry.origin_peer_id)?;
        let repo_scope = ops::local_repo_scope(self.manager.local_repo_name());
        self.manager
            .run_on_primary_local_repo(|db| ops::append_op_to_db(db, entry, &repo_scope))
    }

    pub(crate) fn append_local_op_in_local_repo(
        &self,
        repo_name: &str,
        entry: &LedgerEntry,
    ) -> Result<u64> {
        self.ensure_local_origin(&entry.origin_peer_id)?;
        let repo_scope = ops::local_repo_scope(repo_name);
        self.manager
            .run_on_local_repo(repo_name, |db| ops::append_op_to_db(db, entry, &repo_scope))
    }

    pub(crate) fn append_generated_op(
        &self,
        doc_id: DocId,
        peer_id: PeerId,
        op_entry_builder: impl FnMut(u64) -> LedgerEntry,
    ) -> Result<(u64, u64)> {
        self.ensure_local_origin(&peer_id)?;
        let repo_scope = ops::local_repo_scope(self.manager.local_repo_name());
        self.manager.run_on_primary_local_repo(move |db| {
            ops::append_generated_op(db, doc_id, peer_id, &repo_scope, op_entry_builder)
        })
    }

    pub(crate) fn append_generated_op_in_local_repo(
        &self,
        repo_name: &str,
        doc_id: DocId,
        peer_id: PeerId,
        op_entry_builder: impl FnMut(u64) -> LedgerEntry,
    ) -> Result<(u64, u64)> {
        self.ensure_local_origin(&peer_id)?;
        let repo_scope = ops::local_repo_scope(repo_name);
        self.manager.run_on_local_repo(repo_name, move |db| {
            ops::append_generated_op(db, doc_id, peer_id, &repo_scope, op_entry_builder)
        })
    }

    pub(crate) fn append_generated_client_op_in_local_repo(
        &self,
        repo_name: &str,
        doc_id: DocId,
        peer_id: PeerId,
        client_id: u64,
        client_op_id: u64,
        op_entry_builder: impl FnMut(u64) -> LedgerEntry,
    ) -> Result<(u64, u64)> {
        self.ensure_local_origin(&peer_id)?;
        let repo_scope = ops::local_repo_scope(repo_name);
        self.manager.run_on_local_repo(repo_name, move |db| {
            ops::append_generated_client_op(
                db,
                doc_id,
                peer_id,
                &repo_scope,
                client_id,
                client_op_id,
                op_entry_builder,
            )
        })
    }

    fn ensure_local_origin(&self, peer_id: &PeerId) -> Result<()> {
        if peer_id != self.manager.local_peer_id() {
            anyhow::bail!(
                "Local fact origin {} does not match host identity {}",
                peer_id,
                self.manager.local_peer_id()
            );
        }
        Ok(())
    }
}

impl RepoManager {
    pub(crate) fn authority_storage_runtime(&self) -> AuthorityStorageRuntime<'_> {
        AuthorityStorageRuntime::new(self)
    }

    /// Acquires exact, generation-bound access to one local authority database.
    ///
    /// The returned capability is deliberately non-Clone. Holding it across
    /// unrelated work delays repo quiescence and should be avoided.
    pub fn lease_local_authority(
        &self,
        repo_id: crate::models::RepoId,
    ) -> std::result::Result<RepoAuthorityLease, LocalAuthorityError> {
        let membership = self
            .catalog_membership
            .issue(repo_id)
            .map_err(|_| LocalAuthorityError::NotAdmitted(repo_id))?;
        let lease = match self.local_authority.lease(repo_id) {
            Err(LocalAuthorityError::NotAdmitted(_)) => {
                self.local_authority.admit_existing(repo_id)?
            }
            result => result?,
        };
        self.catalog_membership
            .revalidate(&membership)
            .map_err(|_| LocalAuthorityError::NotAdmitted(repo_id))?;
        Ok(lease)
    }

    pub(crate) fn lease_local_authority_stem(
        &self,
        stem: &str,
    ) -> std::result::Result<RepoAuthorityLease, LocalAuthorityError> {
        let repo_id = uuid::Uuid::parse_str(stem).map_err(|_| {
            LocalAuthorityError::Invariant(format!(
                "local authority selector is not a canonical RepoId: {stem}"
            ))
        })?;
        if repo_id.to_string() != stem {
            return Err(LocalAuthorityError::Invariant(format!(
                "local authority selector is not canonical: {stem}"
            )));
        }
        self.lease_local_authority(repo_id)
    }

    pub(crate) fn with_initial_primary_for_catalog<R>(
        &self,
        repo_id: RepoId,
        inspect: impl FnOnce(&redb::Database) -> std::result::Result<R, LocalAuthorityError>,
    ) -> std::result::Result<R, LocalAuthorityError> {
        if self.local_authority.primary_repo_id() != repo_id {
            return Err(LocalAuthorityError::NotAdmitted(repo_id));
        }
        let prepared = self
            .initial_prepared_authority
            .lock()
            .map_err(|_| LocalAuthorityError::Poisoned)?;
        let prepared = prepared
            .as_ref()
            .ok_or(LocalAuthorityError::NotAdmitted(repo_id))?;
        if prepared.repo_id() != repo_id {
            return Err(LocalAuthorityError::NotAdmitted(repo_id));
        }
        inspect(prepared.db())
    }

    pub(crate) fn bind_local_authority(
        &self,
        repo_id: RepoId,
    ) -> std::result::Result<BoundRepoAuthority, LocalAuthorityError> {
        let membership = self
            .catalog_membership
            .issue(repo_id)
            .map_err(|_| LocalAuthorityError::NotAdmitted(repo_id))?;
        let lease = self.lease_local_authority(repo_id)?;
        drop(lease);
        Ok(self
            .local_authority
            .bind(self.catalog_membership.clone(), membership))
    }

    /// Creates and initializes one exact local authority under this host owner.
    ///
    /// Dynamic repo lifecycle code must use this method instead of composing a
    /// second `RepoManager` for the same ledger directory.
    pub fn create_local_repo_authority(
        &self,
        repo_id: RepoId,
        repo_url: Option<String>,
    ) -> anyhow::Result<(crate::ledger::RepoInfo, PreparedRepoAuthority)> {
        let info = crate::ledger::RepoInfo {
            uuid: repo_id,
            name: repo_id.to_string(),
            url: repo_url.or_else(|| Some(format!("urn:uuid:{repo_id}"))),
        };
        let prepared = self
            .local_authority
            .create_repo_initialized(repo_id, |db| {
                crate::ledger::init::init_core_tables(db)?;
                crate::ledger::source_control::init_tables(db)?;
                Self::initialize_repo_info_in_new_db(db, &info)?;
                Ok(())
            })?;
        Ok((info, prepared))
    }

    pub fn activate_prepared_local_repo_authority(
        &self,
        prepared: PreparedRepoAuthority,
        creation: &crate::ledger::PreparedRepoCreation,
        commit: &crate::ledger::RepoCatalogCreationCommit,
    ) -> std::result::Result<(), LocalAuthorityError> {
        let record = commit.record();
        if prepared.repo_id() != creation.repo_id()
            || record.repo_id() != creation.repo_id()
            || !record.confirms_created(creation.lifecycle_request_id())
            || record.prepared_identity_digest() != creation.prepared_identity().to_hex()
        {
            return Err(LocalAuthorityError::NotAdmitted(prepared.repo_id()));
        }
        prepared.activate(
            &self.local_authority,
            commit.membership(),
            &self.catalog_membership,
        )
    }

    pub fn activate_initial_prepared_local_repo_authority(
        &self,
        creation: &crate::ledger::PreparedRepoCreation,
        commit: &crate::ledger::RepoCatalogCreationCommit,
    ) -> std::result::Result<(), LocalAuthorityError> {
        let repo_id = creation.repo_id();
        let record = commit.record();
        if record.repo_id() != repo_id
            || !record.confirms_created(creation.lifecycle_request_id())
            || record.prepared_identity_digest() != creation.prepared_identity().to_hex()
        {
            return Err(LocalAuthorityError::NotAdmitted(repo_id));
        }
        if self.local_authority.primary_repo_id() != repo_id {
            return Err(LocalAuthorityError::NotAdmitted(repo_id));
        }
        let mut initial = self
            .initial_prepared_authority
            .lock()
            .map_err(|_| LocalAuthorityError::Poisoned)?;
        let Some(prepared) = initial.take() else {
            drop(initial);
            let lease = self.local_authority.lease(repo_id)?;
            drop(lease);
            return Ok(());
        };
        drop(initial);
        prepared.activate(
            &self.local_authority,
            commit.membership(),
            &self.catalog_membership,
        )
    }

    #[cfg(test)]
    pub(crate) fn local_authority_lease_for_test(
        &self,
        repo_id: crate::models::RepoId,
    ) -> anyhow::Result<RepoAuthorityLease> {
        Ok(self.lease_local_authority(repo_id)?)
    }
}
