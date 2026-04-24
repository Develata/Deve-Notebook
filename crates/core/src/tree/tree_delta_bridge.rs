//! plan_ref:
//!   - 06_repository#tree-projection-contract

use super::delta::TreeDelta;
use super::manager::TreeManager;
use crate::models::StructureOp;

/// 将 StructureOp 序列映射为 TreeDelta 增量更新。
///
/// 每个 StructureOp 产生一个对应的 TreeDelta，
/// 由 handler 侧直接广播，避免全量 projection refresh。
pub fn apply_structure_ops(tree: &mut TreeManager, ops: &[StructureOp]) -> Vec<TreeDelta> {
    ops.iter().filter_map(|op| apply_one(tree, op)).collect()
}

fn apply_one(tree: &mut TreeManager, op: &StructureOp) -> Option<TreeDelta> {
    match op {
        StructureOp::CreateFile {
            node_id,
            doc_id,
            parent_id,
            name,
        } => {
            let path = tree.compute_child_path(*parent_id, name);
            Some(tree.add_file(*node_id, path, *parent_id, name.clone(), *doc_id))
        }
        StructureOp::CreateDir {
            node_id,
            parent_id,
            name,
        } => {
            let path = tree.compute_child_path(*parent_id, name);
            Some(tree.add_folder(*node_id, path, *parent_id, name.clone()))
        }
        StructureOp::RenameNode {
            node_id, new_name, ..
        } => {
            if !tree.has_node(*node_id) {
                return None;
            }
            Some(tree.rename_node(*node_id, new_name.clone()))
        }
        StructureOp::MoveNode {
            node_id,
            new_parent_id,
            ..
        } => {
            if !tree.has_node(*node_id) {
                return None;
            }
            Some(tree.move_node(*node_id, *new_parent_id))
        }
        StructureOp::DeleteNode { node_id, .. } => {
            if !tree.has_node(*node_id) {
                return None;
            }
            Some(tree.remove(*node_id))
        }
    }
}
