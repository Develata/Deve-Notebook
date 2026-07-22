// crates\core\src\ledger
//! plan_ref:
//!   - 03_storage/authority#facts-partition
//!   - 03_storage/index#repo-runtime-layout
//!   - 04_repository#repo-catalog-contract
//!
//! # 仓库管理器初始化模块 (RepoManager Initialization)
//!
//! 处理 RepoManager 的初始化逻辑，包括目录结构创建和数据库表初始化。
//!
//! ## 目录结构
//!
//! ```text
//! {ledger_dir}/
//! ├── local/              # 本地权威库 (Store B)
//! │   └── <repo_id>.redb
//! └── remotes/            # 影子库目录 (Store C)
//!     ├── peer_a_name/
//!     │   └── <repo_id>.redb
//!     └── peer_b_name/
//!         └── <repo_id>.redb
//! ```

use anyhow::{Context, Result};
use redb::Database;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use super::RepoManager;
use super::init_reuse::should_reuse_existing_repo;
use super::manager::repo_catalog_entries::redb_repo_entries;
use super::manager::{
    LocalAuthorityDiscovery, LocalAuthorityRuntime, catalog_bootstrap_snapshot_for_ledger,
};
use super::node_check;
use super::schema::*;
use super::source_control;
use crate::models::RepoId;
use crate::utils::fs::checked_exists;

mod catalog;
use catalog::{
    ExistingLocalRepo, scan_cataloged_local_repos, select_existing_local_repo,
    validate_current_cataloged_identity,
};

#[derive(Debug, Clone, Default)]
pub struct RepoInitOptions {
    pub repo_id: Option<uuid::Uuid>,
    pub repo_url: Option<String>,
}

/// 初始化 RepoManager 实例
///
/// 创建账本目录结构，打开/创建本地数据库，并初始化所有必需的表。
///
/// # 参数
///
/// * `ledger_dir` - 账本根目录路径
/// * `snapshot_depth` - 快照保留深度（超出部分会被裁剪）
/// * `repo_name` - 仓库名称（可选，默认为 "default"）
/// * `repo_url` - 仓库 URL（可选，用于区分同名仓库）
///
/// # 错误
///
/// 当目录创建或数据库操作失败时返回错误。
pub fn init(
    ledger_dir: impl AsRef<Path>,
    snapshot_depth: usize,
    repo_name: Option<&str>,
    repo_url: Option<&str>,
) -> Result<RepoManager> {
    init_with_options(
        ledger_dir,
        snapshot_depth,
        repo_name,
        RepoInitOptions {
            repo_id: None,
            repo_url: repo_url.map(str::to_string),
        },
    )
}

pub fn init_with_options(
    ledger_dir: impl AsRef<Path>,
    snapshot_depth: usize,
    repo_name: Option<&str>,
    options: RepoInitOptions,
) -> Result<RepoManager> {
    let ledger_dir = ledger_dir.as_ref().to_path_buf();
    let base_name =
        super::manager::projection_locator::safe_repo_path_segment(repo_name.unwrap_or("default"))?;
    let repo_url = options.repo_url.as_deref();

    // 1. 创建目录结构
    std::fs::create_dir_all(&ledger_dir)
        .with_context(|| format!("无法创建账本目录: {:?}", ledger_dir))?;
    let ledger_dir = canonical_ledger_dir(&ledger_dir)?;

    let local_dir = ledger_dir.join("local");
    std::fs::create_dir_all(&local_dir)
        .with_context(|| format!("无法创建本地库目录: {:?}", local_dir))?;

    let remotes_dir = ledger_dir.join("remotes");
    std::fs::create_dir_all(&remotes_dir)
        .with_context(|| format!("无法创建远端目录: {:?}", remotes_dir))?;

    let host_keys_dir = crate::utils::notegit::host_keys_dir(&ledger_dir);
    let local_peer_id =
        crate::security::load_or_generate_identity_key_at(&host_keys_dir)?.peer_id();

    // 3. Resolve display identity from metadata, then use RepoId as the only physical stem.
    let catalog_snapshot = catalog_bootstrap_snapshot_for_ledger(&ledger_dir)?;
    let discovery = LocalAuthorityDiscovery::new(&ledger_dir);
    let explicit_selection = if let Some(repo_id) = options.repo_id {
        if catalog_snapshot.has_records() && !catalog_snapshot.normal_repo_ids().contains(&repo_id)
        {
            anyhow::bail!(
                "Explicit local RepoId {} is not a durable Normal catalog member",
                repo_id
            );
        }
        let path = local_dir.join(format!("{repo_id}.redb"));
        if checked_exists(&path, "explicit local RepoId during init")? {
            if !catalog_snapshot.has_records() {
                anyhow::bail!(
                    "Uncataloged local authority {} requires explicit ownership repair; normal init cannot resume a prepared artifact",
                    repo_id
                );
            }
            let lease = discovery.lease(repo_id)?;
            let info = RepoManager::read_local_repo_info_from_db(lease.db())?.ok_or_else(|| {
                anyhow::anyhow!(
                    "Broken local repo {} while opening explicit local RepoId: repository metadata missing",
                    repo_id
                )
            })?;
            if info.uuid != repo_id
                || info.name != repo_id.to_string()
                || !should_reuse_existing_repo(repo_url, &info)
            {
                anyhow::bail!(
                    "Existing local RepoId {} metadata does not match explicit init request",
                    repo_id
                );
            }
            Some(ExistingLocalRepo { path, info })
        } else if catalog_snapshot.has_records() {
            anyhow::bail!(
                "Durable Normal local RepoId {} has no canonical authority database",
                repo_id
            );
        } else {
            None
        }
    } else {
        None
    };
    let existing = if options.repo_id.is_none() {
        if catalog_snapshot.has_records() {
            scan_cataloged_local_repos(&local_dir, &discovery, catalog_snapshot.normal_records())?
        } else {
            let uncataloged = redb_repo_entries(
                &local_dir,
                "checking uncataloged local authorities during bootstrap",
            )?;
            if !uncataloged.is_empty() {
                anyhow::bail!(
                    "Uncataloged local authority artifacts require explicit ownership repair; normal init will not admit prepared or orphan databases"
                );
            }
            Vec::new()
        }
    } else {
        Vec::new()
    };
    let selection = if options.repo_id.is_some() {
        explicit_selection.as_ref()
    } else {
        select_existing_local_repo(&existing, &base_name, repo_url, None)?
    };
    if selection.is_none() && catalog_snapshot.has_records() {
        anyhow::bail!(
            "No durable Normal local repository matches the requested selector; physical databases are not admitted outside explicit repair"
        );
    }
    drop(discovery);
    let (_selection_name, repo_uuid, local_authority, initial_prepared_authority) = if let Some(
        selected,
    ) = selection
    {
        let authority = LocalAuthorityRuntime::open_existing(&ledger_dir, selected.info.uuid)
            .with_context(|| format!("无法打开现有数据库: {:?}", selected.path))?;
        let lease = authority.lease_primary()?;
        let record = catalog_snapshot
            .normal_record(selected.info.uuid)
            .ok_or_else(|| anyhow::anyhow!("selected RepoId lost durable Normal membership"))?;
        validate_current_cataloged_identity(&ledger_dir, record, lease.db())?;
        drop(lease);
        (
            selected.info.name.clone(),
            selected.info.uuid,
            authority,
            None,
        )
    } else {
        let display_name = base_name.clone();
        let repo_uuid = options.repo_id.unwrap_or_else(uuid::Uuid::new_v4);
        let db_path = local_dir.join(format!("{}.redb", repo_uuid));
        if checked_exists(&db_path, "UUID-first local database path during init")? {
            anyhow::bail!(
                "Local authority path collision for RepoId {} at {:?}",
                repo_uuid,
                db_path
            );
        }
        let info = super::RepoInfo {
            uuid: repo_uuid,
            name: repo_uuid.to_string(),
            url: options
                .repo_url
                .clone()
                .or_else(|| Some(format!("urn:uuid:{repo_uuid}"))),
        };
        let (authority, prepared) = LocalAuthorityRuntime::prepare_new_initialized(
                &ledger_dir,
                repo_uuid,
                |db| {
                    init_core_tables(db)?;
                    source_control::init_tables(db)?;
                    let report = node_check::check_node_consistency(db)?;
                    if !report.is_clean() {
                        return Err(anyhow::anyhow!(
                            "Node consistency drift detected during prepared init: missing={} orphan={}",
                            report.missing_nodes.len(),
                            report.orphan_nodes.len(),
                        )
                        .into());
                    }
                    super::RepoManager::initialize_repo_info_in_new_db(db, &info)?;
                    Ok(())
                },
            )
            .with_context(|| format!("无法创建本地数据库: {:?}", db_path))?;
        (display_name, repo_uuid, authority, Some(prepared))
    };
    if initial_prepared_authority.is_none() {
        let local_db = local_authority.lease_primary()?;
        super::RepoManager::validate_local_repo_schema(local_db.db())?;
        super::runtime_tables::repair_client_op_index(local_db.db())?;
        source_control::init_tables(local_db.db())?;
        let report = node_check::check_node_consistency(local_db.db())?;
        if !report.is_clean() {
            anyhow::bail!(
                "Node consistency drift detected during init: missing={} orphan={}; run `deve_cli node-check --repair` to repair explicitly",
                report.missing_nodes.len(),
                report.orphan_nodes.len(),
            );
        }
    }

    let catalog_membership = super::manager::CatalogMembershipRuntime::for_ledger(&ledger_dir)?;

    let repo = RepoManager {
        ledger_dir,
        local_peer_id,
        local_authority,
        initial_prepared_authority: std::sync::Mutex::new(initial_prepared_authority),
        shadow_dbs: RwLock::new(HashMap::new()),
        shadow_merge_guard: std::sync::Mutex::new(()),
        snapshot_depth,
        persist_guard: Arc::new(crate::writeback::PersistGuard::new()),
        catalog_membership,
    };
    repo.seed_catalog_membership_from_records()
        .context("Failed to seed local repo catalog membership during init")?;
    if catalog_snapshot.has_records() {
        repo.catalog_membership_runtime()
            .issue(repo_uuid)
            .context("Selected local repo ceased to be a Normal catalog member during init")?;
    }
    repo.repair_remote_repo_catalogs()
        .context("Failed to repair remote repo catalogs during init")?;
    Ok(repo)
}

/// Opens one exact canonical local authority database without creating a local
/// authority database or scanning sibling local databases.
pub(crate) fn init_existing_for_repo_id(
    ledger_dir: impl AsRef<Path>,
    snapshot_depth: usize,
    repo_id: RepoId,
) -> Result<RepoManager> {
    let ledger_dir = canonical_ledger_dir(ledger_dir.as_ref())?;
    let catalog_snapshot = catalog_bootstrap_snapshot_for_ledger(&ledger_dir)?;
    if !catalog_snapshot.has_records() || !catalog_snapshot.normal_repo_ids().contains(&repo_id) {
        anyhow::bail!(
            "Explicit local RepoId {} is not a durable Normal catalog member",
            repo_id
        );
    }
    let local_authority = LocalAuthorityRuntime::open_existing(&ledger_dir, repo_id)
        .with_context(|| format!("Local repo not found for UUID {repo_id}"))?;
    let local_db = local_authority.lease_primary()?;
    let record = catalog_snapshot
        .normal_record(repo_id)
        .ok_or_else(|| anyhow::anyhow!("exact RepoId lost durable Normal membership"))?;
    validate_current_cataloged_identity(&ledger_dir, record, local_db.db())?;

    RepoManager::validate_local_repo_schema(local_db.db())?;
    super::runtime_tables::repair_client_op_index(local_db.db())?;
    source_control::init_tables(local_db.db())?;
    let report = node_check::check_node_consistency(local_db.db())?;
    if !report.is_clean() {
        anyhow::bail!(
            "Node consistency drift detected during init: missing={} orphan={}; run `deve_cli node-check --repair` to repair explicitly",
            report.missing_nodes.len(),
            report.orphan_nodes.len(),
        );
    }

    let host_keys_dir = crate::utils::notegit::host_keys_dir(&ledger_dir);
    let local_peer_id =
        crate::security::load_or_generate_identity_key_at(&host_keys_dir)?.peer_id();
    let catalog_membership = super::manager::CatalogMembershipRuntime::for_ledger(&ledger_dir)?;
    drop(local_db);
    let repo = RepoManager {
        ledger_dir,
        local_peer_id,
        local_authority,
        initial_prepared_authority: std::sync::Mutex::new(None),
        shadow_dbs: RwLock::new(HashMap::new()),
        shadow_merge_guard: std::sync::Mutex::new(()),
        snapshot_depth,
        persist_guard: Arc::new(crate::writeback::PersistGuard::new()),
        catalog_membership,
    };
    repo.seed_catalog_membership_from_records()
        .context("Failed to seed local repo catalog membership during exact repo open")?;
    if catalog_snapshot.has_records() {
        repo.catalog_membership_runtime()
            .issue(repo_id)
            .context("Exact local repo ceased to be a Normal catalog member during init")?;
    }
    repo.repair_remote_repo_catalogs()
        .context("Failed to repair remote repo catalogs during exact repo open")?;
    Ok(repo)
}

/// Composes the host registries for a durable catalog with zero Normal repos.
///
/// The empty runtime is intentionally distinct from repo initialization: it
/// creates host layout and identity only, and never opens or creates a local
/// authority database.
pub(crate) fn init_empty_host(
    ledger_dir: impl AsRef<Path>,
    snapshot_depth: usize,
) -> Result<RepoManager> {
    let ledger_dir = ledger_dir.as_ref().to_path_buf();
    std::fs::create_dir_all(ledger_dir.join("local"))
        .with_context(|| format!("Failed to create empty local authority dir: {ledger_dir:?}"))?;
    std::fs::create_dir_all(ledger_dir.join("remotes"))
        .with_context(|| format!("Failed to create empty shadow authority dir: {ledger_dir:?}"))?;
    let ledger_dir = canonical_ledger_dir(&ledger_dir)?;

    let catalog_snapshot = catalog_bootstrap_snapshot_for_ledger(&ledger_dir)?;
    if !catalog_snapshot.normal_repo_ids().is_empty() {
        anyhow::bail!("empty host initialization requires zero durable Normal local repos");
    }
    let uncataloged = redb_repo_entries(
        &ledger_dir.join("local"),
        "checking empty-host local authority artifacts",
    )?;
    let removed_repo_ids = catalog_snapshot.removed_repo_ids();
    let has_unknown_authority = uncataloged.iter().any(|(_, stem)| {
        uuid::Uuid::parse_str(stem)
            .ok()
            .is_none_or(|repo_id| !removed_repo_ids.contains(&repo_id))
    });
    if has_unknown_authority {
        anyhow::bail!(
            "Uncataloged local authority artifacts require explicit ownership repair; empty host startup will not admit them"
        );
    }

    let host_keys_dir = crate::utils::notegit::host_keys_dir(&ledger_dir);
    let local_peer_id =
        crate::security::load_or_generate_identity_key_at(&host_keys_dir)?.peer_id();
    let catalog_membership = super::manager::CatalogMembershipRuntime::for_ledger(&ledger_dir)?;
    let repo = RepoManager {
        local_authority: LocalAuthorityRuntime::empty(&ledger_dir),
        ledger_dir,
        local_peer_id,
        initial_prepared_authority: std::sync::Mutex::new(None),
        shadow_dbs: RwLock::new(HashMap::new()),
        shadow_merge_guard: std::sync::Mutex::new(()),
        snapshot_depth,
        persist_guard: Arc::new(crate::writeback::PersistGuard::new()),
        catalog_membership,
    };
    repo.seed_catalog_membership_from_records()
        .context("Failed to seed empty local repo catalog membership")?;
    repo.repair_remote_repo_catalogs()
        .context("Failed to repair remote repo catalogs during empty host init")?;
    Ok(repo)
}

fn canonical_ledger_dir(ledger_dir: &Path) -> Result<PathBuf> {
    std::fs::canonicalize(ledger_dir)
        .with_context(|| format!("Failed to canonicalize ledger directory: {ledger_dir:?}"))
}

/// 初始化本地数据库的核心表
///
/// 包括:
/// - `DOCID_TO_PATH`: DocId -> 文件路径 映射
/// - `PATH_TO_DOCID`: 文件路径 -> DocId 映射
/// - `INODE_TO_DOCID`: Inode -> DocId 映射 (用于重命名检测)
/// - `NODEID_TO_META`: NodeId -> NodeMeta 映射
/// - `PATH_TO_NODEID`: Path -> NodeId 映射
/// - `INODE_TO_NODEID`: Inode -> NodeId 映射 (文件节点)
/// - `LEDGER_OPS`: 操作日志表
/// - `DOC_OPS`: 文档操作索引
/// - `NODE_OPS`: 结构事实节点索引
/// - `SNAPSHOT_INDEX`: 快照索引
/// - `SNAPSHOT_DATA`: 快照数据
pub(crate) fn init_core_tables(db: &Database) -> Result<()> {
    let write_txn = db.begin_write()?;
    {
        let _ = write_txn.open_table(DOCID_TO_PATH)?;
        let _ = write_txn.open_table(PATH_TO_DOCID)?;
        let _ = write_txn.open_table(INODE_TO_DOCID)?;
        let _ = write_txn.open_table(NODEID_TO_META)?;
        let _ = write_txn.open_table(PATH_TO_NODEID)?;
        let _ = write_txn.open_table(INODE_TO_NODEID)?;
        let _ = write_txn.open_table(LEDGER_OPS)?;
        let _ = write_txn.open_multimap_table(DOC_OPS)?;
        let _ = write_txn.open_multimap_table(NODE_OPS)?;
        let _ = write_txn.open_table(CLIENT_OP_INDEX)?;
        let _ = write_txn.open_table(PEER_FACT_SEQ)?;
        let _ = write_txn.open_table(PEER_FACT_OPS)?;
        let _ = write_txn.open_table(MERGE_BASE_CHECKPOINT)?;
        let _ = write_txn.open_multimap_table(SNAPSHOT_INDEX)?;
        let _ = write_txn.open_table(SNAPSHOT_DATA)?;
        let _ = write_txn.open_table(REMOTE_IMPORT_SESSIONS)?;
        let _ = write_txn.open_table(REMOTE_IMPORT_RUNTIME)?;
        let _ = write_txn.open_table(PROJECTION_FAULTS)?;
    }
    write_txn.commit()?;
    Ok(())
}
