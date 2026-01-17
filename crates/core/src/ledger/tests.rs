// crates\core\src\ledger
//! # 仓库管理器测试模块 (RepoManager Tests)
//!
//! 包含 RepoManager 的单元测试和集成测试。

use super::*;
use anyhow::Result;
use tempfile::TempDir;

use uuid::Uuid;

/// 测试 RepoManager 初始化
///
/// 验证:
/// - 账本目录正确创建
/// - 本地数据库文件存在 (`local/default.redb`)
/// - 远端目录存在 (`remotes/`)
#[test]
fn test_repo_manager_init() -> Result<()> {
    let tmp_dir = TempDir::new()?;
    let ledger_dir = tmp_dir.path().join("ledger");
    
    let repo = RepoManager::init(&ledger_dir, 10, None)?;
    
    // 验证目录结构
    assert!(ledger_dir.exists());
    assert!(ledger_dir.join("local").join("default.redb").exists());
    assert!(ledger_dir.join("remotes").exists());
    
    Ok(())
}

/// 测试自定义仓库名称初始化
#[test]
fn test_repo_manager_init_custom_name() -> Result<()> {
    let tmp_dir = TempDir::new()?;
    let ledger_dir = tmp_dir.path().join("ledger");
    
    // Initialize with custom name "my_wiki"
    let _repo = RepoManager::init(&ledger_dir, 10, Some("my_wiki"))?;
    
    // Verify file creation
    assert!(ledger_dir.join("local").join("my_wiki.redb").exists());
    
    Ok(())
}

/// 测试本地库与影子库的隔离性
///
/// 验证 Trinity Isolation 架构:
/// - 写入本地库的操作不会出现在影子库
/// - 写入影子库的操作不会出现在本地库
/// - 影子库文件物理隔离存储
#[test]
fn test_local_and_shadow_isolation() -> Result<()> {
    let tmp_dir = TempDir::new()?;
    let ledger_dir = tmp_dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 10, None)?;
    
    let doc_id = DocId::new();
    let peer_id = PeerId::new("peer_mobile");
    let repo_id = Uuid::new_v4(); // Generate a random repo ID for testing remote sync
    
    // 写入本地库 (Active Default Repo)
    let local_entry = LedgerEntry {
        doc_id,
        op: crate::models::Op::Insert { pos: 0, content: "local content".to_string() },
        timestamp: 1000,
    };
    repo.append_local_op(&local_entry)?;
    
    // 写入影子库
    let remote_entry = LedgerEntry {
        doc_id,
        op: crate::models::Op::Insert { pos: 0, content: "remote content".to_string() },
        timestamp: 2000,
    };
    repo.append_remote_op(&peer_id, &repo_id, &remote_entry)?;
    
    // 验证隔离性
    // Local ops using get_local_ops helper (uses default repo id internally)
    let local_ops = repo.get_local_ops(doc_id)?;
    assert_eq!(local_ops.len(), 1);
    
    let remote_ops = repo.get_ops(&RepoType::Remote(peer_id.clone(), repo_id), doc_id)?;
    assert_eq!(remote_ops.len(), 1);
    
    // 验证影子库文件存在
    let shadow_path = ledger_dir.join("remotes")
        .join(peer_id.to_filename())
        .join(format!("{}.redb", repo_id));
    assert!(shadow_path.exists());
    
    Ok(())
}

/// 测试快照裁剪功能
///
/// 验证快照保留深度限制:
/// - 当快照数量超过 `snapshot_depth` 时,旧快照被自动删除
/// - 索引和数据保持一致
#[test]
fn test_snapshot_pruning() -> Result<()> {
    let tmp_dir = TempDir::new()?;
    let ledger_dir = tmp_dir.path().join("ledger");
    
    // 设置快照深度为 2
    let repo = RepoManager::init(&ledger_dir, 2, None)?;
    let doc_id = DocId::new();
    
    // 保存 3 个快照
    repo.save_snapshot(doc_id, 1, "Snap 1")?;
    repo.save_snapshot(doc_id, 2, "Snap 2")?;
    repo.save_snapshot(doc_id, 3, "Snap 3")?; // 这应该触发 seq=1 的裁剪
    
    // 验证裁剪结果
    let read_txn = repo.local_db.begin_read()?;
    let index = read_txn.open_multimap_table(SNAPSHOT_INDEX)?;
    let data = read_txn.open_table(SNAPSHOT_DATA)?;
    
    let mut seqs = Vec::new();
    for item in index.get(doc_id.as_u128())? {
        seqs.push(item?.value());
    }
    seqs.sort();
    
    assert_eq!(seqs, vec![2, 3], "快照索引应该只包含 2 和 3");
    assert!(data.get(1)?.is_none(), "快照 1 的数据应该被删除");
    assert!(data.get(2)?.is_some(), "快照 2 的数据应该存在");
    assert!(data.get(3)?.is_some(), "快照 3 的数据应该存在");
    
    Ok(())
}
