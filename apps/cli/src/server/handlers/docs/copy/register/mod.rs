//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!   - 03_storage/authority#repo-mutation-publication-gate
//!
//! Two-phase copied-document registration. Filesystem enumeration and source
//! reconstruction happen before the repo permit; exact source identity/content
//! is revalidated while the authority mutations are serialized.

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::handlers::docs::copy_utils::{collect_dirs, collect_md_files};
use crate::server::handlers::docs::file_register::create_file_from_content;
use crate::server::repo_mutation::{MutationExecution, MutationPublication};
use crate::server::repo_scope::{ResolvedRepo, local_repo_root};
use deve_core::models::DocId;
use deve_core::state;
use std::path::Path;
use std::sync::Arc;

mod path;

use path::map_dest_rel;

#[derive(Clone, Copy)]
pub(super) struct CopyRegisterCtx<'a> {
    pub state: &'a Arc<AppState>,
    pub ch: &'a DualChannel,
    pub scope: &'a ResolvedRepo,
    pub scope_nonce: Option<u64>,
}

pub(super) struct CopyRegistrationPlan {
    directories: Vec<String>,
    files: Vec<CopyFilePlan>,
}

struct CopyFilePlan {
    source_path: String,
    source_doc_id: DocId,
    destination_path: String,
    content: String,
}

pub(super) fn prepare_registration(
    ctx: CopyRegisterCtx<'_>,
    src: &Path,
    src_path: &str,
    destination_path: &str,
) -> anyhow::Result<CopyRegistrationPlan> {
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
        files.push(CopyFilePlan {
            destination_path: map_dest_rel(&source_path, src_path, destination_path)?,
            source_path,
            source_doc_id,
            content,
        });
    }
    Ok(CopyRegistrationPlan { directories, files })
}

pub(super) fn prepare_single_file_registration(
    ctx: CopyRegisterCtx<'_>,
    source_path: String,
    source_doc_id: DocId,
    destination_path: String,
) -> anyhow::Result<CopyRegistrationPlan> {
    let content = reconstructed_content(ctx, source_doc_id)?;
    Ok(CopyRegistrationPlan {
        directories: Vec::new(),
        files: vec![CopyFilePlan {
            source_path,
            source_doc_id,
            destination_path,
            content,
        }],
    })
}

pub(super) fn commit_registration(
    ctx: CopyRegisterCtx<'_>,
    plan: &CopyRegistrationPlan,
) -> MutationExecution<Vec<DocId>, anyhow::Error> {
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
        let current_content = match reconstructed_content(ctx, current_doc_id) {
            Ok(content) => content,
            Err(error) => return failed(error, committed, ctx.scope.repo_id),
        };
        if current_content != file.content {
            return failed(
                anyhow::anyhow!("Copy source content changed: {}", file.source_path),
                committed,
                ctx.scope.repo_id,
            );
        }
        match create_file_from_content(
            ctx.state,
            ctx.scope,
            &file.destination_path,
            &file.content,
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

    MutationExecution::committed(
        created_docs,
        MutationPublication::document_recovery(
            ctx.scope.repo_id,
            deve_core::protocol::DocumentRecoveryScope::CurrentDocument,
        ),
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
