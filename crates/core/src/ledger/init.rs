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
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, RwLock};

use super::RepoManager;
use super::database::{cached_database, cached_or_create_database, register_database};
use super::init_reuse::should_reuse_existing_repo;
use super::manager::repair_local_repo_metadata;
use super::manager::repo_catalog_entries::redb_repo_entries;
use super::node_check;
use super::schema::*;
use super::source_control;
use crate::utils::fs::checked_exists;

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
    let existing = scan_local_repo_catalog(&local_dir)?;
    let selection = select_existing_local_repo(&existing, &base_name, repo_url, options.repo_id)?;
    let (final_name, repo_uuid, local_db, is_new_repo) = if let Some(selected) = selection {
        let db = cached_database(&selected.path)
            .with_context(|| format!("无法打开现有数据库以检查元数据: {:?}", selected.path))?;
        (selected.info.name.clone(), selected.info.uuid, db, false)
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
        let db = cached_or_create_database(&db_path)
            .with_context(|| format!("无法创建本地数据库: {:?}", db_path))?;
        (display_name, repo_uuid, db, true)
    };
    let execution_name = repo_uuid.to_string();
    register_database(
        &local_dir.join(format!("{}.redb", execution_name)),
        local_db.clone(),
    )?;

    // 4. 初始化核心表
    if is_new_repo {
        init_core_tables(local_db.as_ref())?;
    } else {
        super::RepoManager::validate_local_repo_schema(local_db.as_ref())?;
        super::runtime_tables::repair_client_op_index(local_db.as_ref())?;
    }

    // 5. 初始化 Source Control 表
    source_control::init_tables(local_db.as_ref())?;

    // 6. Node 表一致性检查（repair 需显式触发）
    let report = node_check::check_node_consistency(local_db.as_ref())?;
    if !report.is_clean() {
        anyhow::bail!(
            "Node consistency drift detected during init: missing={} orphan={}; run `deve_cli node-check --repair` to repair explicitly",
            report.missing_nodes.len(),
            report.orphan_nodes.len(),
        );
    }

    // 7. 写入 Metadata (如果是新库，或者旧库缺失)
    if is_new_repo {
        let info = super::RepoInfo {
            uuid: repo_uuid,
            name: final_name.clone(),
            url: options
                .repo_url
                .clone()
                .or_else(|| Some(format!("urn:uuid:{}", repo_uuid))),
        };
        super::RepoManager::initialize_repo_info_in_new_db(local_db.as_ref(), &info)?;
    }

    repair_local_repo_metadata(&ledger_dir, &execution_name, local_db.as_ref(), false, None)?;
    let catalog_membership = super::manager::CatalogMembershipRuntime::for_ledger(&ledger_dir)?;

    let repo = RepoManager {
        ledger_dir,
        local_peer_id,
        local_db,
        local_repo_name: execution_name,
        extra_local_dbs: RwLock::new(HashMap::new()),
        repaired_local_runtime_tables: RwLock::new(HashSet::new()),
        shadow_dbs: RwLock::new(HashMap::new()),
        shadow_merge_guard: std::sync::Mutex::new(()),
        snapshot_depth,
        persist_guard: Arc::new(crate::writeback::PersistGuard::new()),
        catalog_membership,
    };
    repo.repair_remote_repo_catalogs()
        .context("Failed to repair remote repo catalogs during init")?;
    Ok(repo)
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
fn init_core_tables(db: &Database) -> Result<()> {
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

struct ExistingLocalRepo {
    path: std::path::PathBuf,
    info: super::RepoInfo,
}

fn scan_local_repo_catalog(local_dir: &Path) -> Result<Vec<ExistingLocalRepo>> {
    let mut repos = Vec::new();
    let mut ids = HashMap::new();
    let mut urls = HashMap::new();
    for (path, stem) in redb_repo_entries(local_dir, "initializing local repo")? {
        let info = super::RepoManager::read_required_local_repo_info_from_path(
            &path,
            &stem,
            "initializing local repo",
        )
        .map_err(|err| {
            anyhow::anyhow!(
                "Broken local repo {} while initializing catalog: {}",
                stem,
                err
            )
        })?;
        let expected_stem = info.uuid.to_string();
        if stem != expected_stem {
            anyhow::bail!(
                "Broken v4 local repo {} while initializing catalog: physical stem must equal RepoId {}",
                stem,
                info.uuid
            );
        }
        if let Some(owner) = ids.insert(info.uuid, stem.clone()) {
            anyhow::bail!(
                "Broken local catalog: duplicate RepoId {} at {} and {}",
                info.uuid,
                owner,
                stem
            );
        }
        if let Some(url) = &info.url
            && let Some(owner) = urls.insert(url.clone(), stem.clone())
        {
            anyhow::bail!(
                "Broken local catalog: duplicate repository URL {} at {} and {}",
                url,
                owner,
                stem
            );
        }
        repos.push(ExistingLocalRepo { path, info });
    }
    Ok(repos)
}

fn select_existing_local_repo<'a>(
    repos: &'a [ExistingLocalRepo],
    requested_name: &str,
    requested_url: Option<&str>,
    requested_id: Option<uuid::Uuid>,
) -> Result<Option<&'a ExistingLocalRepo>> {
    if let Some(repo_id) = requested_id {
        if let Some(repo) = repos.iter().find(|repo| repo.info.uuid == repo_id) {
            if repo.info.name != requested_name
                || !should_reuse_existing_repo(requested_url, &repo.info)
            {
                anyhow::bail!(
                    "Existing local RepoId {} metadata does not match explicit init request",
                    repo_id
                );
            }
            return Ok(Some(repo));
        }
        if let Some(repo) = repos.iter().find(|repo| {
            repo.info.name == requested_name
                && should_reuse_existing_repo(requested_url, &repo.info)
        }) {
            anyhow::bail!(
                "explicit repo-id init fails closed: repository selector {} resolves to existing RepoId {}, not requested RepoId {}",
                requested_name,
                repo.info.uuid,
                repo_id
            );
        }
        return Ok(None);
    }
    let matches = repos
        .iter()
        .filter(|repo| {
            repo.info.name == requested_name
                && should_reuse_existing_repo(requested_url, &repo.info)
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [] => Ok(None),
        [repo] => Ok(Some(*repo)),
        _ => anyhow::bail!(
            "Ambiguous local repository init selector {} matched {} RepoIds; pass an explicit RepoId or unique URL",
            requested_name,
            matches.len()
        ),
    }
}
