// crates\core\src\ledger\shadow
//! # 影子库管理层 (Shadow Management)
//!
//! **架构作用**:
//! 处理影子库的生命周期：创建、打开、表初始化和磁盘扫描。
//!
//! **核心功能清单**:
//! - `ensure_shadow_db`: 确保指定 Peer 的数据库存在且表结构完整。
//! - `list_shadows_on_disk`: 扫描磁盘发现已有的影子库。
//!
//! **类型**: Core MUST (核心必选)

use crate::ledger::schema::*;
use crate::models::{PeerId, RepoId};
use anyhow::{Context, Result};
use redb::Database;
use std::collections::HashMap;
use std::path::Path;
use std::sync::RwLock;

/// 确保指定 Peer 的特定影子库已加载。
pub fn ensure_shadow_db(
    remotes_dir: &Path,
    shadow_dbs: &RwLock<HashMap<PeerId, HashMap<RepoId, Database>>>,
    peer_id: &PeerId,
    repo_id: &RepoId,
) -> Result<()> {
    // Check if already loaded (Read Lock)
    {
        let dbs = shadow_dbs.read().unwrap();
        if let Some(repos) = dbs.get(peer_id)
            && repos.contains_key(repo_id) {
                return Ok(());
            }
    }

    // Acquire Write Lock
    let mut dbs = shadow_dbs.write().unwrap();

    // Double-Check: Check again under Write Lock
    // Another thread might have created it while we waited for the lock
    if let Some(repos) = dbs.get(peer_id)
        && repos.contains_key(repo_id) {
            return Ok(());
        }

    // Create peer directory: remotes/<peer_id>/
    let peer_dir = remotes_dir.join(peer_id.to_filename());
    std::fs::create_dir_all(&peer_dir)
        .with_context(|| format!("Failed to create shadow directory for peer: {}", peer_id))?;

    // Create or open the shadow database: remotes/<peer_id>/<repo_id>.redb
    let db_path = peer_dir.join(format!("{}.redb", repo_id));
    let db = Database::create(&db_path).with_context(|| {
        format!(
            "Failed to create shadow database for peer {} repo {}",
            peer_id, repo_id
        )
    })?;

    // Initialize tables
    let write_txn = db.begin_write()?;
    {
        let _ = write_txn.open_table(LEDGER_OPS)?;
        let _ = write_txn.open_multimap_table(DOC_OPS)?;
        let _ = write_txn.open_multimap_table(SNAPSHOT_INDEX)?;
        let _ = write_txn.open_table(SNAPSHOT_DATA)?;

        // Metadata tables (for remote file listing)
        let _ = write_txn.open_table(PATH_TO_DOCID)?;
        let _ = write_txn.open_table(DOCID_TO_PATH)?;
        let _ = write_txn.open_table(INODE_TO_DOCID)?;
    }
    write_txn.commit()?;

    // Store in map
    dbs.entry(peer_id.clone())
        .or_default()
        .insert(*repo_id, db);

    Ok(())
}

/// 扫描磁盘上所有影子库文件夹并返回 PeerId 列表。
pub fn list_shadows_on_disk(remotes_dir: &Path) -> Result<Vec<PeerId>> {
    let mut peers = Vec::new();

    if remotes_dir.exists() {
        tracing::info!("Scanning remotes dir: {:?}", remotes_dir);
        let entries = std::fs::read_dir(remotes_dir)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            tracing::info!("Found entry: {:?}", path);
            if path.is_dir() {
                if let Some(stem) = path.file_name() {
                    // Assuming dir name is PeerId (or encoded filename)
                    peers.push(PeerId::new(&*stem.to_string_lossy()));
                }
            } else {
                tracing::warn!("Entry is not a directory: {:?}", path);
            }
        }
    } else {
        tracing::warn!("Remotes dir not found: {:?}", remotes_dir);
    }

    Ok(peers)
}
