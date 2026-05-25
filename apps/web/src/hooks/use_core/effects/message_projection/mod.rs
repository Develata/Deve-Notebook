//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!   - 04_repository#tree-projection-contract
//!
mod doc;
mod tree;

pub use self::doc::handle_doc_list;
pub use self::tree::handle_tree_update;
