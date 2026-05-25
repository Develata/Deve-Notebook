// crates\core\src\ledger\shadow
//! plan_ref:
//!   - 04_repository#repo-catalog-contract
//!   - 04_repository#repo-scope-runtime
//!   - 03_storage/index#repo-runtime-layout
//!   - 03_storage/authority#facts-partition
//!
//! # 影子库管理层 (Shadow Management)
//!
//! **架构作用**:
//! 处理影子库的生命周期：创建、打开、表初始化和磁盘扫描。
//!
//! **核心功能清单**:
//! - `ensure_shadow_db`: 确保指定 Peer 的数据库存在且表结构完整。
//! - `load_shadow_db`: 按显式路径打开/初始化影子库。
//! - `list_shadows_on_disk`: 扫描磁盘发现已有的影子库。
//!
//! **类型**: Core MUST (核心必选)

use crate::ledger::database::cached_or_create_database;
use crate::ledger::manager::types::RepoInfo;
use crate::ledger::schema::*;
use crate::models::{PeerId, RepoId};
use anyhow::{Context, Result};
use redb::{Database, ReadableTable};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

type ShadowRepoMap = HashMap<PeerId, HashMap<RepoId, Arc<Database>>>;

/// 确保指定 Peer 的特定影子库已加载。
///
/// Invariants:
/// - 一旦显式 `repo_id` 已知，runtime 创建的 shadow DB 必须至少写入最小 `RepoInfo`。
/// - `REPO_METADATA` 缺失只允许来自显式构造的 legacy/broken 测试输入，不应由正常运行时路径制造。
pub fn ensure_shadow_db(
    remotes_dir: &Path,
    shadow_dbs: &RwLock<HashMap<PeerId, HashMap<RepoId, Arc<Database>>>>,
    peer_id: &PeerId,
    repo_id: &RepoId,
) -> Result<()> {
    let db_path = remotes_dir
        .join(peer_id.to_filename())
        .join(format!("{}.redb", repo_id));
    load_shadow_db(remotes_dir, shadow_dbs, peer_id, repo_id, &db_path)
}

pub fn load_shadow_db(
    remotes_dir: &Path,
    shadow_dbs: &RwLock<HashMap<PeerId, HashMap<RepoId, Arc<Database>>>>,
    peer_id: &PeerId,
    repo_id: &RepoId,
    db_path: &Path,
) -> Result<()> {
    // Check if already loaded (Read Lock)
    {
        let dbs = read_shadow_dbs(shadow_dbs)?;
        if let Some(repos) = dbs.get(peer_id)
            && repos.contains_key(repo_id)
        {
            return Ok(());
        }
    }

    // Acquire Write Lock
    let mut dbs = write_shadow_dbs(shadow_dbs)?;

    // Double-Check: Check again under Write Lock
    // Another thread might have created it while we waited for the lock
    if let Some(repos) = dbs.get(peer_id)
        && repos.contains_key(repo_id)
    {
        return Ok(());
    }

    // Create peer directory: remotes/<peer_id>/
    let peer_dir = remotes_dir.join(peer_id.to_filename());
    std::fs::create_dir_all(&peer_dir)
        .with_context(|| format!("Failed to create shadow directory for peer: {}", peer_id))?;

    let db = cached_or_create_database(db_path).with_context(|| {
        format!(
            "Failed to create shadow database for peer {} repo {}",
            peer_id, repo_id
        )
    })?;

    // Initialize tables
    let write_txn = db.begin_write()?;
    {
        let mut repo_meta = write_txn.open_table(REPO_METADATA)?;
        if repo_meta.get(&0)?.is_none() {
            let info = RepoInfo {
                uuid: *repo_id,
                name: repo_id.to_string(),
                url: None,
            };
            let bytes = bincode::serialize(&info)?;
            repo_meta.insert(&0, bytes.as_slice())?;
        }
        let _ = write_txn.open_table(LEDGER_OPS)?;
        let _ = write_txn.open_multimap_table(DOC_OPS)?;
        let _ = write_txn.open_multimap_table(NODE_OPS)?;
        let _ = write_txn.open_table(CLIENT_OP_INDEX)?;
        let _ = write_txn.open_table(NODE_PEER_SEQ)?;
        let _ = write_txn.open_multimap_table(SNAPSHOT_INDEX)?;
        let _ = write_txn.open_table(SNAPSHOT_DATA)?;

        // Projection tables (for remote file listing / tree view)
        let _ = write_txn.open_table(PATH_TO_DOCID)?;
        let _ = write_txn.open_table(DOCID_TO_PATH)?;
        let _ = write_txn.open_table(INODE_TO_DOCID)?;
        let _ = write_txn.open_table(NODEID_TO_META)?;
        let _ = write_txn.open_table(PATH_TO_NODEID)?;
        let _ = write_txn.open_table(INODE_TO_NODEID)?;
    }
    write_txn.commit()?;

    // Store in map
    dbs.entry(peer_id.clone()).or_default().insert(*repo_id, db);

    Ok(())
}

fn read_shadow_dbs<'a>(
    shadow_dbs: &'a RwLock<ShadowRepoMap>,
) -> Result<RwLockReadGuard<'a, ShadowRepoMap>> {
    shadow_dbs
        .read()
        .map_err(|_| anyhow::anyhow!("Shadow DB registry lock poisoned"))
}

fn write_shadow_dbs<'a>(
    shadow_dbs: &'a RwLock<ShadowRepoMap>,
) -> Result<RwLockWriteGuard<'a, ShadowRepoMap>> {
    shadow_dbs
        .write()
        .map_err(|_| anyhow::anyhow!("Shadow DB registry lock poisoned"))
}

/// 扫描磁盘上所有影子库文件夹并返回 PeerId 列表。
pub fn list_shadows_on_disk(remotes_dir: &Path) -> Result<Vec<PeerId>> {
    let mut peers = Vec::new();
    match remotes_dir.try_exists() {
        Ok(true) => {}
        Ok(false) => {
            anyhow::bail!(
                "Broken remote repo catalog: remote repo directory missing at {:?}",
                remotes_dir
            );
        }
        Err(err) => {
            return Err(err).with_context(|| {
                format!(
                    "Failed to stat remote repo directory while listing shadows on disk: {:?}",
                    remotes_dir
                )
            });
        }
    }
    if !std::fs::metadata(remotes_dir)
        .with_context(|| {
            format!(
                "Failed to read remote repo directory metadata while listing shadows on disk: {:?}",
                remotes_dir
            )
        })?
        .is_dir()
    {
        anyhow::bail!(
            "Broken remote repo catalog: expected directory at {:?}",
            remotes_dir
        );
    }
    for entry in std::fs::read_dir(remotes_dir)? {
        let entry = entry?;
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            return Err(anyhow::anyhow!(
                "Broken shadow peer entry {:?} while listing shadows on disk: invalid directory name",
                path
            ));
        };
        if name == ".invalid" {
            continue;
        }
        if name.starts_with('.') || name.is_empty() {
            return Err(anyhow::anyhow!(
                "Broken shadow peer entry {:?} while listing shadows on disk: unexpected hidden directory",
                path
            ));
        }
        if !entry.file_type()?.is_dir() {
            return Err(anyhow::anyhow!(
                "Broken shadow peer entry {:?} while listing shadows on disk: expected directory",
                path
            ));
        }
        peers.push(PeerId::new(name));
    }

    peers.sort();
    Ok(peers)
}
