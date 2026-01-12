//! # 影子库管理模块 (Shadow Repository)
//! 
//! 管理远端 Peer 的影子数据库 (Store C)。
//! 实现懒加载和磁盘扫描。

use anyhow::{Result, Context};
use redb::{Database, ReadableMultimapTable, ReadableTable};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use crate::models::PeerId;
use crate::ledger::schema::*;

/// 确保指定 Peer 的影子库已加载。
pub fn ensure_shadow_db(
    remotes_dir: &Path,
    shadow_dbs: &RwLock<HashMap<PeerId, Database>>,
    peer_id: &PeerId
) -> Result<()> {
    // Check if already loaded
    {
        let dbs = shadow_dbs.read().unwrap();
        if dbs.contains_key(peer_id) {
            return Ok(());
        }
    }
    
    // Create or open the shadow database
    let db_path = remotes_dir.join(format!("{}.redb", peer_id.to_filename()));
    let db = Database::create(&db_path)
        .with_context(|| format!("Failed to create shadow database for peer: {}", peer_id))?;
    
    // Initialize tables
    let write_txn = db.begin_write()?;
    {
        let _ = write_txn.open_table(LEDGER_OPS)?;
        let _ = write_txn.open_multimap_table(DOC_OPS)?;
        let _ = write_txn.open_multimap_table(SNAPSHOT_INDEX)?;
        let _ = write_txn.open_table(SNAPSHOT_DATA)?;
        // Note: Shadow DBs don't need path mappings (metadata stays in local)
    }
    write_txn.commit()?;
    
    // Store in map
    let mut dbs = shadow_dbs.write().unwrap();
    dbs.insert(peer_id.clone(), db);
    
    Ok(())
}

/// 扫描磁盘上所有影子库文件并返回 PeerId 列表。
pub fn list_shadows_on_disk(remotes_dir: &Path) -> Result<Vec<PeerId>> {
    let mut peers = Vec::new();
    
    if remotes_dir.exists() {
        for entry in std::fs::read_dir(remotes_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "redb") {
                if let Some(stem) = path.file_stem() {
                    peers.push(PeerId::new(stem.to_string_lossy().to_string()));
                }
            }
        }
    }
    
    Ok(peers)
}
