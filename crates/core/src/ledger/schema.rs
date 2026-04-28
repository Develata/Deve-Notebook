// crates\core\src\ledger
//! plan_ref:
//!   - 04_storage#facts-partition
//!   - 04_storage#repo-runtime-layout

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

// Sequence (u64) -> LedgerEntry (Bytes)
pub const LEDGER_OPS: TableDefinition<u64, &[u8]> = TableDefinition::new("ledger_ops");

// DocId (u128) -> Vec<u64> (Sequence Numbers) - Secondary Index
pub const DOC_OPS: MultimapTableDefinition<u128, u64> = MultimapTableDefinition::new("doc_ops");

// NodeId (u128) -> Vec<u64> (Structure Fact Sequence Numbers)
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

// Metadata Key (u8) -> Metadata Value (Bytes - JSON/Bincode)
// Key 0: RepoInfo (UUID, Name, URL)
pub const REPO_METADATA: TableDefinition<u8, &[u8]> = TableDefinition::new("repo_metadata");

// (DocId (u128), PeerId (&str)) -> MaxSeq (u64)
// Used for atomic sequence generation and O(1) retrieval.
pub const PEER_DOC_SEQ: TableDefinition<(u128, &str), u64> = TableDefinition::new("peer_doc_seq");

// (NodeId (u128), PeerId (&str)) -> MaxSeq (u64)
pub const NODE_PEER_SEQ: TableDefinition<(u128, &str), u64> = TableDefinition::new("node_peer_seq");

// (DocId, ClientId, ClientOpId) -> GlobalSeq
// 浏览器写入去重索引，用于 reconnect 后安全重发。
pub const CLIENT_OP_INDEX: TableDefinition<(u128, u64, u64), u64> =
    TableDefinition::new("client_op_index");

// Path String -> PendingFsEntry (Bytes - JSON)
// 存储 Watcher 检测到但用户尚未确认的文件系统变更
// Key: 相对路径, Value: PendingFsEntry 序列化字节
pub const PENDING_FS_OPS: TableDefinition<&str, &[u8]> = TableDefinition::new("pending_fs_ops");
pub const PENDING_FS_DOC_INDEX: MultimapTableDefinition<u128, &str> =
    MultimapTableDefinition::new("pending_fs_doc_index");
pub const STAGED_DOC_INDEX: MultimapTableDefinition<u128, &str> =
    MultimapTableDefinition::new("staged_doc_index");
