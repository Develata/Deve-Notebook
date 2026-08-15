//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!   - 03_storage/authority#repo-mutation-publication-gate
//!
//! Two-phase copied-document registration. Filesystem enumeration and source
//! reconstruction happen before the repo permit; exact source identity/content
//! is revalidated while the authority mutations are serialized.

use crate::server::AppState;
use crate::server::handlers::docs::copy_utils::{
    PreparedAssetCopy, apply_prepared_asset_copies, collect_dirs, collect_md_files,
    prepare_dir_asset_copies,
};
use crate::server::handlers::docs::file_register::create_file_from_patch;
use crate::server::repo_mutation::{MutationExecution, MutationPublication};
use crate::server::repo_scope::{ResolvedRepo, local_repo_root};
use deve_core::ledger::range;
use deve_core::models::{DocId, Op};
use deve_core::state;
use std::path::Path;
use std::sync::Arc;

mod path;

use path::map_dest_rel;

#[derive(Clone, Copy)]
pub(super) struct CopyRegisterCtx<'a> {
    pub state: &'a Arc<AppState>,
    pub scope: &'a ResolvedRepo,
}

pub(super) struct CopyRegistrationPlan {
    expected_ledger_head: u64,
    directories: Vec<String>,
    files: Vec<CopyFilePlan>,
    assets: Vec<PreparedAssetCopy>,
}

struct CopyFilePlan {
    source_path: String,
    source_doc_id: DocId,
    destination_path: String,
    content: String,
    patch: Vec<Op>,
}

pub(super) fn prepare_registration(
    ctx: CopyRegisterCtx<'_>,
    src: &Path,
    dst: &Path,
    src_path: &str,
    destination_path: &str,
) -> anyhow::Result<CopyRegistrationPlan> {
    let expected_ledger_head = ctx
        .state
        .repo
        .run_on_local_repo(&ctx.scope.repo_name, range::get_max_seq)?;
    let base = local_repo_root(ctx.state, ctx.scope)?;
    let mut source_directories = collect_dirs(src, &base)?;
    source_directories.sort_by_key(|path| path.matches('/').count());
    let directories = source_directories
        .into_iter()
        .map(|path| map_dest_rel(&path, src_path, destination_path))
        .collect::<anyhow::Result<Vec<_>>>()?;

    let mut files = Vec::new();
    for source_path in collect_md_files(src, &base)? {
        let source_doc_id = ctx
            .state
            .repo
            .get_tracked_docid_in_local_repo(&ctx.scope.repo_name, &source_path)?
            .ok_or_else(|| anyhow::anyhow!("Source doc not tracked: {source_path}"))?;
        let content = reconstructed_content(ctx, source_doc_id)?;
        let patch = state::compute_diff("", &content)?;
        files.push(CopyFilePlan {
            destination_path: map_dest_rel(&source_path, src_path, destination_path)?,
            source_path,
            source_doc_id,
            content,
            patch,
        });
    }
    let assets = prepare_dir_asset_copies(src, dst)?;
    ensure_preparation_head_unchanged(ctx, expected_ledger_head)?;
    Ok(CopyRegistrationPlan {
        expected_ledger_head,
        directories,
        files,
        assets,
    })
}

pub(super) fn prepare_single_file_registration(
    ctx: CopyRegisterCtx<'_>,
    source_path: String,
    source_doc_id: DocId,
    destination_path: String,
) -> anyhow::Result<CopyRegistrationPlan> {
    let expected_ledger_head = ctx
        .state
        .repo
        .run_on_local_repo(&ctx.scope.repo_name, range::get_max_seq)?;
    let content = reconstructed_content(ctx, source_doc_id)?;
    let patch = state::compute_diff("", &content)?;
    ensure_preparation_head_unchanged(ctx, expected_ledger_head)?;
    Ok(CopyRegistrationPlan {
        expected_ledger_head,
        directories: Vec::new(),
        files: vec![CopyFilePlan {
            source_path,
            source_doc_id,
            destination_path,
            content,
            patch,
        }],
        assets: Vec::new(),
    })
}

pub(super) fn commit_registration(
    ctx: CopyRegisterCtx<'_>,
    plan: &CopyRegistrationPlan,
) -> MutationExecution<Vec<DocId>, anyhow::Error> {
    let observed_head = match ctx
        .state
        .repo
        .run_on_local_repo(&ctx.scope.repo_name, range::get_max_seq)
    {
        Ok(head) => head,
        Err(error) => return MutationExecution::not_committed(error),
    };
    if observed_head != plan.expected_ledger_head {
        return MutationExecution::not_committed(anyhow::anyhow!(
            "copy source changed while waiting for mutation permit: expected head {}, observed {}",
            plan.expected_ledger_head,
            observed_head
        ));
    }
    let mut committed = false;
    let mut created_docs = Vec::with_capacity(plan.files.len());

    for destination in &plan.directories {
        match ctx.state.repo.apply_dir_create_structure_in_local_repo(
            &ctx.scope.repo_name,
            destination,
            "local_copy",
        ) {
            Ok((_node_id, ops)) => committed |= !ops.is_empty(),
            Err(error) => return failed(error, committed, ctx.scope.repo_id),
        }
    }

    for file in &plan.files {
        let current_doc_id = match ctx
            .state
            .repo
            .get_tracked_docid_in_local_repo(&ctx.scope.repo_name, &file.source_path)
        {
            Ok(Some(doc_id)) => doc_id,
            Ok(None) => {
                return failed(
                    anyhow::anyhow!("Copy source disappeared: {}", file.source_path),
                    committed,
                    ctx.scope.repo_id,
                );
            }
            Err(error) => return failed(error, committed, ctx.scope.repo_id),
        };
        if current_doc_id != file.source_doc_id {
            return failed(
                anyhow::anyhow!("Copy source identity changed: {}", file.source_path),
                committed,
                ctx.scope.repo_id,
            );
        }
        match create_file_from_patch(
            ctx.state,
            ctx.scope,
            &file.destination_path,
            &file.content,
            &file.patch,
            "local_copy",
        ) {
            Ok((doc_id, _ops)) => {
                committed = true;
                created_docs.push(doc_id);
            }
            // The helper spans structure/content transactions and projection
            // writeback; conservatively classify an opaque failure as partial.
            Err(error) => {
                return MutationExecution::committed_partial(
                    error,
                    MutationPublication::document_recovery(
                        ctx.scope.repo_id,
                        deve_core::protocol::DocumentRecoveryScope::CurrentDocument,
                    ),
                );
            }
        }
    }

    if let Err(error) = apply_prepared_asset_copies(&plan.assets) {
        return MutationExecution::committed_partial(
            error.into(),
            recovery_publication(ctx.scope.repo_id),
        );
    }
    let consistency = match ctx.state.repo.run_on_local_repo(
        &ctx.scope.repo_name,
        deve_core::ledger::node_check::check_node_consistency,
    ) {
        Ok(report) => report,
        Err(error) => {
            return MutationExecution::committed_partial(
                error,
                recovery_publication(ctx.scope.repo_id),
            );
        }
    };
    if !consistency.is_clean() {
        return MutationExecution::committed_partial(
            anyhow::anyhow!(
                "Node consistency dirty after copy: missing={} orphan={}",
                consistency.missing_nodes.len(),
                consistency.orphan_nodes.len()
            ),
            recovery_publication(ctx.scope.repo_id),
        );
    }

    MutationExecution::committed(created_docs, recovery_publication(ctx.scope.repo_id))
}

fn ensure_preparation_head_unchanged(
    ctx: CopyRegisterCtx<'_>,
    expected: u64,
) -> anyhow::Result<()> {
    let observed = ctx
        .state
        .repo
        .run_on_local_repo(&ctx.scope.repo_name, range::get_max_seq)?;
    if observed != expected {
        anyhow::bail!(
            "copy source changed during preparation: expected head {}, observed {}",
            expected,
            observed
        );
    }
    Ok(())
}

fn recovery_publication(repo_id: deve_core::models::RepoId) -> MutationPublication {
    MutationPublication::document_recovery(
        repo_id,
        deve_core::protocol::DocumentRecoveryScope::CurrentDocument,
    )
}

fn failed(
    error: anyhow::Error,
    committed: bool,
    repo_id: deve_core::models::RepoId,
) -> MutationExecution<Vec<DocId>, anyhow::Error> {
    if committed {
        MutationExecution::committed_partial(
            error,
            MutationPublication::document_recovery(
                repo_id,
                deve_core::protocol::DocumentRecoveryScope::CurrentDocument,
            ),
        )
    } else {
        MutationExecution::not_committed(error)
    }
}

fn reconstructed_content(ctx: CopyRegisterCtx<'_>, doc_id: DocId) -> anyhow::Result<String> {
    let ops = ctx
        .state
        .repo
        .get_local_ops_in_local_repo(&ctx.scope.repo_name, doc_id)?;
    let entries: Vec<_> = ops.into_iter().map(|(_, entry)| entry).collect();
    Ok(state::reconstruct_content(&entries))
}
