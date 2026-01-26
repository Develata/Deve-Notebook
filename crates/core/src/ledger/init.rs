// crates\core\src\ledger
//! # 仓库管理器初始化模块 (RepoManager Initialization)
//!
//! 处理 RepoManager 的初始化逻辑，包括目录结构创建和数据库表初始化。
//!
//! ## 目录结构
//!
//! ```text
//! {ledger_dir}/
//! ├── local/              # 本地权威库 (Store B)
//! │   └── repo_name_1.redb
//! │   └── repo_name_3.redb
//! │   └── repo_name_4.redb
//! └── remotes/            # 影子库目录 (Store C)
//!     ├── peer_a_name/
//!     │   └── repo_name_1.redb
//!     │   └── repo_name_2.redb
//!     │   └── repo_name_5.redb
//!     └── peer_b_name/
//!         └── repo_name_1.redb
//!         └── repo_name_2.redb
//!         └── repo_name_3.redb
//! ```

use anyhow::{Context, Result};
use redb::Database;
use std::collections::HashMap;
use std::path::Path;
use std::sync::RwLock;

use super::schema::*;
use super::source_control;
use super::RepoManager;

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
    let ledger_dir = ledger_dir.as_ref().to_path_buf();

    // 1. 创建目录结构
    std::fs::create_dir_all(&ledger_dir)
        .with_context(|| format!("无法创建账本目录: {:?}", ledger_dir))?;

    let local_dir = ledger_dir.join("local");
    std::fs::create_dir_all(&local_dir)
        .with_context(|| format!("无法创建本地库目录: {:?}", local_dir))?;

    let remotes_dir = ledger_dir.join("remotes");
    std::fs::create_dir_all(&remotes_dir)
        .with_context(|| format!("无法创建远端目录: {:?}", remotes_dir))?;

    // 2. 准备仓库标识
    let base_name = repo_name.unwrap_or("default");

    // 3. 碰撞检测与处理 (Collision Handling)
    // 策略: 检查文件是否存在 -> 若存在，检查 URL 是否匹配 -> 若不匹配，重命名尝试 (name-1)
    let mut final_name = base_name.to_string();
    let mut counter = 0;
    let local_db;
    let mut is_new_repo = false;

    loop {
        if counter > 0 {
            final_name = format!("{}-{}", base_name, counter);
        }
        let db_path = local_dir.join(format!("{}.redb", final_name));

        if db_path.exists() {
            // 尝试打开现有库检查 Metadata
            let db = Database::create(&db_path)
                .with_context(|| format!("无法打开现有数据库以检查元数据: {:?}", db_path))?;

            // 检查 URL
            let read_txn = db.begin_read()?;
            match read_txn.open_table(REPO_METADATA) {
                Ok(table) => {
                    if let Some(guard) = table.get(&0)? {
                        let val = guard.value();
                        let info: super::RepoInfo = bincode::deserialize(val)?;

                        // 如果 URL 匹配 (或都为 None)，则确认为同一仓库
                        // 注意:这里简化处理，如果输入的 URL 为 None，我们假设匹配任何（或者生成新的？）
                        // 根据需求: "Logical Identity: URL is strictly distinguishing"
                        // 如果输入 URL 为 None, 我们通常是在"打开默认库"，所以匹配。
                        // 如果输入 URL 有值，必须严格匹配。
                        let match_url = match (repo_url, &info.url) {
                            (Some(u1), Some(u2)) => u1 == u2,
                            (None, _) => true, // 没有指定 URL，默认复用同名库
                            (Some(_), None) => false, // 指定了 URL 但库里没有 (视为不匹配，或是旧库升级?) -> 安全起见视为不匹配
                        };

                        if match_url {
                            local_db = db;
                            break;
                        } else {
                            // URL 不匹配，这是另一个同名仓库 -> 继续循环尝试下一个名字
                            counter += 1;
                            continue;
                        }
                    }
                }
                Err(_) => {
                    // 没有元数据表 (旧版本?) -> 视为匹配 (复用)
                    // 或者视为脏数据? 假设复用。
                    local_db = db;
                    break;
                }
            }
        } else {
            // 文件不存在，创建新库
            local_db = Database::create(&db_path)
                .with_context(|| format!("无法创建本地数据库: {:?}", db_path))?;
            is_new_repo = true;
            break;
        }
    }

    // 4. 初始化核心表
    init_core_tables(&local_db)?;

    // 5. 初始化 Source Control 表
    source_control::init_tables(&local_db)?;

    // 6. 写入 Metadata (如果是新库，或者旧库缺失)
    // 即使是旧库，如果缺失也可以补全? 还是保持原样?
    // 这里我们只在 is_new_repo 时写入，或者做一下检查
    if is_new_repo {
        let repo_uuid = uuid::Uuid::new_v4();
        let info = super::RepoInfo {
            uuid: repo_uuid,
            name: final_name.clone(),
            url: repo_url
                .map(|s| s.to_string())
                .or_else(|| Some(format!("urn:uuid:{}", repo_uuid))),
        };
        let write_txn = local_db.begin_write()?;
        {
            let mut table = write_txn.open_table(REPO_METADATA)?;
            let bytes = bincode::serialize(&info)?;
            table.insert(&0, bytes.as_slice())?;
        }
        write_txn.commit()?;
    }

    Ok(RepoManager {
        ledger_dir,
        local_db,
        local_repo_name: final_name,
        extra_local_dbs: RwLock::new(HashMap::new()),
        shadow_dbs: RwLock::new(HashMap::new()),
        snapshot_depth,
    })
}

/// 初始化本地数据库的核心表
///
/// 包括:
/// - `DOCID_TO_PATH`: DocId -> 文件路径 映射
/// - `PATH_TO_DOCID`: 文件路径 -> DocId 映射
/// - `INODE_TO_DOCID`: Inode -> DocId 映射 (用于重命名检测)
/// - `LEDGER_OPS`: 操作日志表
/// - `DOC_OPS`: 文档操作索引
/// - `SNAPSHOT_INDEX`: 快照索引
/// - `SNAPSHOT_DATA`: 快照数据
fn init_core_tables(db: &Database) -> Result<()> {
    let write_txn = db.begin_write()?;
    {
        let _ = write_txn.open_table(DOCID_TO_PATH)?;
        let _ = write_txn.open_table(PATH_TO_DOCID)?;
        let _ = write_txn.open_table(INODE_TO_DOCID)?;
        let _ = write_txn.open_table(LEDGER_OPS)?;
        let _ = write_txn.open_multimap_table(DOC_OPS)?;
        let _ = write_txn.open_multimap_table(SNAPSHOT_INDEX)?;
        let _ = write_txn.open_table(SNAPSHOT_DATA)?;
    }
    write_txn.commit()?;
    Ok(())
}
