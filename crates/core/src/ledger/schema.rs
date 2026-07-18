// crates\core\src\ledger
//! plan_ref:
//!   - 03_storage/authority#facts-partition
//!   - 03_storage/authority#redb-schema-version-contract
//!   - 03_storage/authority#projection-fault-recovery-table
//!   - 03_storage/index#repo-runtime-layout

use redb::{MultimapTableDefinition, TableDefinition};

// DocId (u128) -> Path String
pub const DOCID_TO_PATH: TableDefinition<u128, &str> = TableDefinition::new("docid_to_path");

// Path String -> DocId (u128)
pub const PATH_TO_DOCID: TableDefinition<&str, u128> = TableDefinition::new("path_to_docid");

// FileNodeId (u128) -> DocId (u128) - For Rename Detection
pub const INODE_TO_DOCID: TableDefinition<u128, u128> = TableDefinition::new("inode_to_docid");

// NodeId (u128) -> NodeMeta (Bytes)
pub const NODEID_TO_META: TableDefinition<u128, &[u8]> = TableDefinition::new("nodeid_to_meta");

// Path String -> NodeId (u128)
pub const PATH_TO_NODEID: TableDefinition<&str, u128> = TableDefinition::new("path_to_nodeid");

// FileNodeId (u128) -> NodeId (u128) - For Rename Detection (File only)
pub const INODE_TO_NODEID: TableDefinition<u128, u128> = TableDefinition::new("inode_to_nodeid");

// GlobalSeq storage key (u64) -> LedgerEntry (Bytes)
pub const LEDGER_OPS: TableDefinition<u64, &[u8]> = TableDefinition::new("ledger_ops");

// DocId (u128) -> Vec<u64> (GlobalSeq storage keys) - Secondary Index
pub const DOC_OPS: MultimapTableDefinition<u128, u64> = MultimapTableDefinition::new("doc_ops");

// NodeId (u128) -> Vec<u64> (Structure Fact GlobalSeq storage keys)
pub const NODE_OPS: MultimapTableDefinition<u128, u64> = MultimapTableDefinition::new("node_ops");

// DocId (u128) -> (Sequence (u64), Content (String)) - We might store multiple snapshots?
// Ideally: (DocId, Seq) -> Content.
// But Redb doesn't support Composite Key easily without serialization.
// Let's use Multimap: DocId -> (Seq, ContentBytes) ?
// Or separate table: SNAPSHOTS: Table<u64, &[u8]> where key is Sequence? No.
// Let's use: DocId -> SnapshotData. But we need multiple?
// "Snapshot Depth" implies multiple.
// Let's use MultimapTableDefinition<u128, Vec<u8>>? No, we need to sort by Seq.
// Maybe: SNAPSHOT_INDEX: Multimap<u128, u64> (DocId -> Seq)
//        SNAPSHOT_DATA: Table<u64, &[u8]> (Seq -> Data)
pub const SNAPSHOT_INDEX: MultimapTableDefinition<u128, u64> =
    MultimapTableDefinition::new("snapshot_index");
pub const SNAPSHOT_DATA: TableDefinition<u64, &[u8]> = TableDefinition::new("snapshot_data");

// Metadata Key (u8) -> Metadata Value (Bytes - JSON/Postcard)
pub const REPO_INFO_METADATA_KEY: u8 = 0;
pub const REPO_SCHEMA_VERSION_METADATA_KEY: u8 = 1;
pub const REDB_SCHEMA_VERSION: u16 = 4;

// Key 0: RepoInfo (UUID, Name, URL)
// Key 1: REDB_SCHEMA_VERSION
pub const REPO_METADATA: TableDefinition<u8, &[u8]> = TableDefinition::new("repo_metadata");

// Local-authority Remote Import workflow. Shadow databases deliberately do not own these tables.
pub(crate) const REMOTE_IMPORT_SESSIONS: TableDefinition<u128, &[u8]> =
    TableDefinition::new("remote_import_sessions");
pub(crate) const REMOTE_IMPORT_RUNTIME: TableDefinition<u8, &[u8]> =
    TableDefinition::new("remote_import_runtime");

// Repo-local, host-only recovery evidence. This is not a Ledger Fact table and is absent from
// remote shadows. The fixed key is a domain-separated project-owned SHA-256 identity.
pub(crate) const PROJECTION_FAULTS: TableDefinition<[u8; 32], &[u8]> =
    TableDefinition::new("projection_faults");

// Physical PeerId (&str) -> repo-scoped max PeerFactSeq (u64).
pub const PEER_FACT_SEQ: TableDefinition<&str, u64> = TableDefinition::new("peer_fact_seq_v3");

// (Physical PeerId (&str), PeerFactSeq (u64)) -> GlobalSeq storage key (u64).
pub const PEER_FACT_OPS: TableDefinition<(&str, u64), u64> =
    TableDefinition::new("peer_fact_ops_v3");

// (Source physical PeerId, DocId) -> encoded MergeBaseCheckpoint.
pub const MERGE_BASE_CHECKPOINT: TableDefinition<(&str, u128), &[u8]> =
    TableDefinition::new("merge_base_checkpoint_v3");

// (ClientId, ClientOpId) -> GlobalSeq
// 浏览器写入去重索引，用于 reconnect 后安全重发。
pub const CLIENT_OP_INDEX: TableDefinition<(u64, u64), u64> =
    TableDefinition::new("client_op_index_v2");

// Path String -> PendingFsEntry (Bytes - JSON)
// 存储 Watcher 检测到但用户尚未确认的文件系统变更
// Key: 相对路径, Value: PendingFsEntry 序列化字节
pub const PENDING_FS_OPS: TableDefinition<&str, &[u8]> = TableDefinition::new("pending_fs_ops");
pub const PENDING_FS_DOC_INDEX: MultimapTableDefinition<u128, &str> =
    MultimapTableDefinition::new("pending_fs_doc_index");
pub const STAGED_DOC_INDEX: MultimapTableDefinition<u128, &str> =
    MultimapTableDefinition::new("staged_doc_index");
