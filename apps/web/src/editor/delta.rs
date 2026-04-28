//! plan_ref:
//!   - 03_rendering#large-document-runtime
//!   - 16_web_thin_client_ledger#web-edit-intent
//!
use deve_core::models::Op;

/// Delta 结构 (从 JS 传入)
///
/// 说明：
/// - `from/to` 为 UTF-16 code unit 索引（与 JS/CodeMirror 一致）
#[derive(serde::Deserialize, Debug)]
pub struct Delta {
    pub from: usize,
    pub to: usize,
    pub insert: String,
}

impl Delta {
    /// 将 Delta 转换为单个 Op (⚠️ Replace 时仅返回 Delete，丢弃 Insert)
    ///
    /// **已废弃**: 请使用 `to_ops()` 代替，它能正确处理 Replace = Delete + Insert
    #[allow(dead_code)]
    #[deprecated(note = "Replace 场景会丢弃 Insert，请改用 to_ops()")]
    pub fn to_op(&self) -> Option<Op> {
        let delete_len = self.to.saturating_sub(self.from);
        let has_delete = delete_len > 0;
        let has_insert = !self.insert.is_empty();
        let pos = to_u32(self.from)?;
        let len = if has_delete { to_u32(delete_len)? } else { 0 };

        match (has_delete, has_insert) {
            (true, true) => Some(Op::Delete { pos, len }),
            (true, false) => Some(Op::Delete { pos, len }),
            (false, true) => Some(Op::Insert {
                pos,
                content: self.insert.clone().into(),
            }),
            (false, false) => None,
        }
    }

    /// 将 Delta 转换为 Op 列表 (处理 Replace 情况)
    pub fn to_ops(&self) -> Vec<Op> {
        let delete_len = self.to.saturating_sub(self.from);
        let has_delete = delete_len > 0;
        let has_insert = !self.insert.is_empty();
        let pos = match to_u32(self.from) {
            Some(v) => v,
            None => return Vec::new(),
        };
        let len = match to_u32(delete_len) {
            Some(v) => v,
            None => return Vec::new(),
        };

        let mut ops = Vec::new();

        if has_delete {
            ops.push(Op::Delete { pos, len });
        }

        if has_insert {
            ops.push(Op::Insert {
                pos,
                content: self.insert.clone().into(),
            });
        }

        ops
    }
}

fn to_u32(value: usize) -> Option<u32> {
    u32::try_from(value).ok()
}
