use crate::models::{Op, LedgerEntry};
use anyhow::Result;

pub fn reconstruct_content(ops: &[LedgerEntry]) -> String {
    let mut content = String::new();
    
    // Sort logic? We assume ops are in sequence order from get_ops
    // But Ops themselves might be concurrent in a real CRDT.
    // For Phase 0, we assume linear history from a single device (mostly).
    // If we use Loro later, it handles this.
    // For now, naive string manipulation.
    
    for entry in ops {
        match &entry.op {
            Op::Insert { pos, content: text } => {
                if *pos >= content.len() {
                    content.push_str(text);
                } else {
                    content.insert_str(*pos, text);
                }
            }
            Op::Delete { pos, len } => {
                if *pos < content.len() {
                    let end = std::cmp::min(pos + len, content.len());
                    content.drain(*pos..end);
                }
            }
        }
    }
    
    content
}

pub fn compute_diff(old: &str, new: &str) -> Vec<Op> {
    use dissimilar::Chunk;
    let chunks = dissimilar::diff(old, new);
    let mut ops = Vec::new();
    let mut pos = 0;
    
    for chunk in chunks {
        match chunk {
            Chunk::Equal(text) => {
                pos += text.len();
            }
            Chunk::Insert(text) => {
                ops.push(Op::Insert {
                    pos,
                    content: text.to_string(),
                });
                pos += text.len();
            }
            Chunk::Delete(text) => {
                ops.push(Op::Delete {
                    pos,
                    len: text.len(),
                });
                // Do not advance pos, because we deleted content so the "next" character
                // shifts left to the current pos.
            }
        }
    }
    ops
}
