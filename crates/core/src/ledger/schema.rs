use redb::{TableDefinition, MultimapTableDefinition};

// DocId (u128) -> Path String
pub const DOCID_TO_PATH: TableDefinition<u128, &str> = TableDefinition::new("docid_to_path");

// Path String -> DocId (u128)
pub const PATH_TO_DOCID: TableDefinition<&str, u128> = TableDefinition::new("path_to_docid");

// FileNodeId (u128) -> DocId (u128) - For Rename Detection
pub const INODE_TO_DOCID: TableDefinition<u128, u128> = TableDefinition::new("inode_to_docid");

// Sequence (u64) -> LedgerEntry (Bytes)
pub const LEDGER_OPS: TableDefinition<u64, &[u8]> = TableDefinition::new("ledger_ops");

// DocId (u128) -> Vec<u64> (Sequence Numbers) - Secondary Index
pub const DOC_OPS: MultimapTableDefinition<u128, u64> = MultimapTableDefinition::new("doc_ops");

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
pub const SNAPSHOT_INDEX: MultimapTableDefinition<u128, u64> = MultimapTableDefinition::new("snapshot_index");
pub const SNAPSHOT_DATA: TableDefinition<u64, &[u8]> = TableDefinition::new("snapshot_data");

