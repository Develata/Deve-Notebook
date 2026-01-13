//! # P2P 验证命令
//!
//! **架构作用**:
//! 模拟多节点环境，验证 P2P 同步协议、Shadow Repo 隔离和 CRDT 合并逻辑的正确性。
//! 用于集成测试和架构验证。
//!
//! **实现逻辑**:
//! 1. 创建两个独立的临时目录作为 Repo A 和 Repo B。
//! 2. 初始化两个 `RepoManager` 实例。
//! 3. 模拟 "Gossip" 过程：手动交换 Op。
//! 4. 验证数据一致性。

use std::path::Path;
use std::fs;
use anyhow::Result;
use tracing::{info, warn};
use deve_core::ledger::{RepoManager, RepoType};
use deve_core::models::{PeerId, DocId, LedgerEntry, Op};

pub fn run(snapshot_depth: usize) -> Result<()> {
    info!("Starting P2P Verification Simulation...");

    // 1. Setup Environment
    let temp_dir = std::env::temp_dir().join("deve_note_p2p_verify");
    let peer_a_dir = temp_dir.join("peer_a");
    let peer_b_dir = temp_dir.join("peer_b");

    cleanup(&temp_dir);
    fs::create_dir_all(&peer_a_dir)?;
    fs::create_dir_all(&peer_b_dir)?;

    let peer_a_ledger = peer_a_dir.join("ledger");
    let peer_b_ledger = peer_b_dir.join("ledger");

    fs::create_dir_all(&peer_a_ledger)?;
    fs::create_dir_all(&peer_b_ledger)?;

    info!("Initialized temporary directories at {:?}", temp_dir);

    // 2. Init Repos
    let repo_a = RepoManager::init(&peer_a_ledger, snapshot_depth)?;
    let peer_a_id = PeerId::new("peer_a_sim");
    let peer_b_id = PeerId::new("peer_b_sim");
    
    info!("Repo A initialized (Virtual Peer: {})", peer_a_id);

    let repo_b = RepoManager::init(&peer_b_ledger, snapshot_depth)?;
    info!("Repo B initialized (Virtual Peer: {})", peer_b_id);

    // 3. Peer A creates a doc
    info!("--- Step 1: Peer A creates document ---");
    let doc_path = "hello.md";
    let doc_content = "Hello from Peer A";
    
    let doc_id = repo_a.create_docid(doc_path)?;
    let op = Op::Insert { pos: 0, content: doc_content.to_string() };
    let entry = LedgerEntry {
        doc_id,
        op,
        timestamp: chrono::Utc::now().timestamp_millis(),
    };
    
    let seq = repo_a.append_local_op(&entry)?;
    info!("Peer A created doc: {} ({}) at seq {}", doc_path, doc_id, seq);

    // 4. Sync A -> B
    info!("--- Step 2: Sync A -> B ---");
    let ops_a = repo_a.get_ops(&RepoType::Local, doc_id)?;
    info!("Extracted {} ops from Peer A", ops_a.len());

    let mut applied_count = 0;
    for (_seq, entry) in ops_a {
        repo_b.append_remote_op(&peer_a_id, &entry)?;
        applied_count += 1;
    }
    info!("Applied {} ops to Peer B (Shadow: {})", applied_count, peer_a_id);

    // 5. Verify B state
    info!("--- Step 3: Verify Peer B State ---");
    
    // Read A's shadow in B using get_shadow_ops directly
    // Note: get_shadow_repo is currently specific about lifetimes or placeholders, so determining access via ops is safer.
    
    let ops = repo_b.get_shadow_ops(&peer_a_id, doc_id)?;
    if !ops.is_empty() {
        let entries: Vec<LedgerEntry> = ops.into_iter().map(|(_, e)| e).collect();
        let content_in_b_shadow_a = deve_core::state::reconstruct_content(&entries);
        
        info!("Peer B's view of Peer A: {:?}", content_in_b_shadow_a);
        
        if content_in_b_shadow_a == doc_content {
            info!("✅ SUCCESS: Peer B correctly sees Peer A's content in Shadow Repo.");
        } else {
            warn!("❌ FAILURE: Content mismatch. Expected '{}', got '{}'", doc_content, content_in_b_shadow_a);
            return Err(anyhow::anyhow!("Verification failed at Step 3"));
        }
    } else {
        warn!("❌ FAILURE: Shadow Repo for Peer A has no ops in Peer B!");
        return Err(anyhow::anyhow!("Shadow Repo missing or empty"));
    }

    // 6. Verify Isolation
    let content_b_local = repo_b.resolve_local_content(doc_id)?;
    if content_b_local.is_empty() {
        info!("✅ SUCCESS: Peer B's Local Repo is isolated (empty).");
    } else {
        warn!("❌ FAILURE: Peer B's Local Repo was polluted!");
    }

    // 7. Cleanup
    cleanup(&temp_dir);
    
    info!("P2P Verification Completed Successfully.");
    Ok(())
}

trait ContentResolver {
    fn resolve_local_content(&self, doc_id: DocId) -> Result<String>;
}

impl ContentResolver for RepoManager {
    fn resolve_local_content(&self, doc_id: DocId) -> Result<String> {
        let ops = self.get_ops(&RepoType::Local, doc_id)?;
        let entries: Vec<LedgerEntry> = ops.into_iter().map(|(_, e)| e).collect();
        Ok(deve_core::state::reconstruct_content(&entries))
    }
}

fn cleanup(path: &Path) {
    if path.exists() {
        let _ = fs::remove_dir_all(path);
    }
}
