// crates/core/src/ledger/tests.rs
//! # 仓库管理器测试模块 (RepoManager Tests)
//!
//! 包含 RepoManager 的单元测试和集成测试。

use super::*;
use crate::ledger::schema::{DOCID_TO_PATH, PATH_TO_DOCID};
use crate::ledger::{node_check, node_meta};
use crate::models::{DocId, LedgerEntry, PeerId, RepoType};
use anyhow::Result;
use tempfile::TempDir;
use uuid::Uuid;

fn seed_metadata_only_doc(repo: &RepoManager, path: &str) -> Result<DocId> {
    let doc_id = DocId::new();
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        let write_txn = db.begin_write()?;
        {
            let mut p2d = write_txn.open_table(PATH_TO_DOCID)?;
            let mut d2p = write_txn.open_table(DOCID_TO_PATH)?;
            p2d.insert(path, doc_id.as_u128())?;
            d2p.insert(doc_id.as_u128(), path)?;
        }
        write_txn.commit()?;
        Ok(())
    })?;
    Ok(doc_id)
}

/// 测试 RepoManager 初始化
///
/// 验证:
/// - 账本目录正确创建
/// - 本地数据库文件使用 UUID-first stem
/// - 远端目录存在 (`remotes/`)
#[test]
fn test_repo_manager_init() -> Result<()> {
    let tmp_dir = TempDir::new()?;
    let ledger_dir = tmp_dir.path().join("ledger");

    let repo = RepoManager::init(&ledger_dir, 10, None, None)?;

    // 验证目录结构
    assert!(ledger_dir.exists());
    let repo_id = repo.get_repo_info()?.expect("repo info").uuid;
    assert!(
        ledger_dir
            .join("local")
            .join(format!("{repo_id}.redb"))
            .exists()
    );
    assert!(ledger_dir.join("remotes").exists());

    Ok(())
}

/// 测试自定义仓库名称初始化
#[test]
fn test_repo_manager_init_custom_name() -> Result<()> {
    let tmp_dir = TempDir::new()?;
    let ledger_dir = tmp_dir.path().join("ledger");

    // Initialize with custom name "my_wiki"
    let repo = RepoManager::init(&ledger_dir, 10, Some("my_wiki"), None)?;

    // Verify file creation
    let info = repo.get_repo_info()?.expect("repo info");
    assert_eq!(info.name, "my_wiki");
    assert!(
        ledger_dir
            .join("local")
            .join(format!("{}.redb", info.uuid))
            .exists()
    );

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
    let repo = RepoManager::init(&ledger_dir, 10, None, None)?;

    let doc_id = DocId::new();
    let local_peer_id = repo.local_peer_id().clone();
    let peer_id = PeerId::new("peer_mobile");
    let repo_id = Uuid::new_v4();

    // 写入本地库 (Active Default Repo)
    let local_entry = LedgerEntry::new_content(
        doc_id,
        crate::models::Op::Insert {
            pos: 0,
            content: "local content".into(),
        },
        1000,
        local_peer_id,
        1,
        None,
        None,
    );
    repo.append_local_op(&local_entry)?;

    // 写入影子库
    let remote_entry = LedgerEntry::new_content(
        doc_id,
        crate::models::Op::Insert {
            pos: 0,
            content: "remote content".into(),
        },
        2000,
        peer_id.clone(),
        1,
        None,
        None,
    );
    repo.append_remote_op(&peer_id, &repo_id, &remote_entry)?;

    // 验证隔离性
    let local_ops = repo.get_local_ops(doc_id)?;
    assert_eq!(local_ops.len(), 1);

    let remote_ops = repo.get_ops(&RepoType::Remote(peer_id.clone(), repo_id), doc_id)?;
    assert_eq!(remote_ops.len(), 1);

    // 验证影子库文件存在：运行时必须先经 UUID 恢复 execution-safe selector。
    let shadow_path = repo
        .resolve_remote_repo_entry_by_id(&peer_id, repo_id)?
        .expect("shadow entry")
        .path;
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
    let repo = RepoManager::init(&ledger_dir, 2, None, None)?;
    let doc_id = DocId::new();
    let peer_id = repo.local_peer_id().clone();

    let ops = [(1, "a"), (2, "ab"), (3, "abc")];

    for (seq, content) in ops.iter() {
        let entry = LedgerEntry::new_content(
            doc_id,
            crate::models::Op::Insert {
                pos: (seq - 1) as u32,
                content: content.chars().last().unwrap().to_string().into(),
            },
            1000 + (*seq as i64),
            peer_id.clone(),
            *seq,
            None,
            None,
        );
        repo.append_local_op(&entry)?;
        repo.save_snapshot(doc_id, *seq, content)?;
    }

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

#[test]
fn snapshot_rejects_middle_content_mismatch() -> Result<()> {
    let tmp_dir = TempDir::new()?;
    let ledger_dir = tmp_dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 2, None, None)?;
    let doc_id = DocId::new();
    let content = format!("{}ledger-middle{}", "a".repeat(1024), "z".repeat(1024));
    let candidate = format!("{}forged-middle{}", "a".repeat(1024), "z".repeat(1024));
    assert_eq!(content.len(), candidate.len());

    let entry = LedgerEntry::new_content(
        doc_id,
        crate::models::Op::Insert {
            pos: 0,
            content: content.clone().into(),
        },
        1,
        repo.local_peer_id().clone(),
        1,
        None,
        None,
    );
    let seq = repo.append_local_op(&entry)?;

    let error = repo
        .save_snapshot(doc_id, seq, &candidate)
        .expect_err("same-length candidate with a different middle must fail");
    assert!(error.to_string().contains("Snapshot verification failed"));
    assert!(repo.load_latest_snapshot(doc_id)?.is_none());
    Ok(())
}

#[test]
fn test_node_migration_and_consistency() -> Result<()> {
    let tmp_dir = TempDir::new()?;
    let ledger_dir = tmp_dir.path().join("ledger");
    let (repo, _repo_id) = crate::test_support::init_cataloged_repo_with_depth(
        &ledger_dir,
        &tmp_dir.path().join("notes"),
        2,
    )?;

    let path = "notes/alpha.md";
    let doc_id = seed_metadata_only_doc(&repo, path)?;
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        node_meta::ensure_file_node(db, path, doc_id)?;
        Ok(())
    })?;

    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        let report = node_check::check_node_consistency(db)?;
        assert!(report.is_clean());
        Ok(())
    })?;

    Ok(())
}

#[test]
fn test_dir_node_creation() -> Result<()> {
    let tmp_dir = TempDir::new()?;
    let ledger_dir = tmp_dir.path().join("ledger");
    let (repo, _repo_id) = crate::test_support::init_cataloged_repo_with_depth(
        &ledger_dir,
        &tmp_dir.path().join("notes"),
        2,
    )?;

    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        let dir_id = node_meta::create_dir_node(db, "projects/math")?;
        let meta = node_meta::get_node_meta(db, dir_id)?.expect("dir meta missing");
        assert_eq!(meta.path, "projects/math");
        assert!(meta.parent_id.is_some());
        Ok(())
    })?;

    Ok(())
}

#[test]
fn test_node_repair_missing() -> Result<()> {
    let tmp_dir = TempDir::new()?;
    let ledger_dir = tmp_dir.path().join("ledger");
    let (repo, _repo_id) = crate::test_support::init_cataloged_repo_with_depth(
        &ledger_dir,
        &tmp_dir.path().join("notes"),
        2,
    )?;

    let path = "notes/repair.md";
    let doc_id = seed_metadata_only_doc(&repo, path)?;

    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        node_meta::remove_node_by_path(db, path)?;
        let report = node_check::repair_missing_nodes(db)?;
        assert!(report.missing_nodes.is_empty());
        let meta = node_meta::get_node_meta(db, crate::models::NodeId::from_doc_id(doc_id))?
            .expect("node meta should exist after repair");
        assert_eq!(meta.path, path);
        Ok(())
    })?;

    Ok(())
}
