// apps/web/src/editor/ffi.rs
//! # FFI Bindings (JavaScript 互操作)
//!
//! 定义与 JavaScript (CodeMirror adapter) 交互的外部函数接口。
//!
//! ## 性能优化 (v4)
//! - `setupCodeMirror` 现在接收 Delta 回调 (JSON 字符串)，而不是全文回调
//! - 避免了每次按键时的 JS->WASM 全文拷贝
//! - 添加了 `destroyEditor` 用于清理资源

use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
unsafe extern "C" {
    /// 初始化 CodeMirror 编辑器
    ///
    /// `on_delta`: 接收 JSON 格式的 Delta 数组: `[{from, to, insert}, ...]`
    pub fn setupCodeMirror(element: &web_sys::HtmlElement, on_delta: &Closure<dyn FnMut(String)>);

    /// 销毁编辑器实例，释放资源
    pub fn destroyEditor();

    /// 应用远程快照 (全量替换)
    pub fn applyRemoteContent(text: &str);

    /// 应用远程操作 (增量)
    pub fn applyRemoteOp(op_json: &str);

    /// 获取当前编辑器内容
    pub fn getEditorContent() -> String;

    /// 滚动到指定行
    #[wasm_bindgen(js_name = scrollGlobal)]
    pub fn scroll_global(line: usize);

    /// 设置只读状态
    #[wasm_bindgen(js_name = setReadOnly)]
    pub fn set_read_only(read_only: bool);
}

/// Delta 结构 (从 JS 传入)
#[derive(serde::Deserialize, Debug)]
pub struct Delta {
    pub from: usize,
    pub to: usize,
    pub insert: String,
}

/// 将 Delta 转换为 Op
impl Delta {
    pub fn to_op(&self) -> Option<deve_core::models::Op> {
        let delete_len = self.to.saturating_sub(self.from);
        let has_delete = delete_len > 0;
        let has_insert = !self.insert.is_empty();

        match (has_delete, has_insert) {
            (true, true) => {
                // Replace = Delete + Insert (返回 Delete，Insert 在下一次处理)
                // 为简化，这里将 Replace 视为先删除后插入
                // 调用方需要处理这种情况
                Some(deve_core::models::Op::Delete {
                    pos: self.from,
                    len: delete_len,
                })
            }
            (true, false) => Some(deve_core::models::Op::Delete {
                pos: self.from,
                len: delete_len,
            }),
            (false, true) => Some(deve_core::models::Op::Insert {
                pos: self.from,
                content: self.insert.clone(),
            }),
            (false, false) => None,
        }
    }

    /// 将 Delta 转换为 Op 列表 (处理 Replace 情况)
    pub fn to_ops(&self) -> Vec<deve_core::models::Op> {
        let delete_len = self.to.saturating_sub(self.from);
        let has_delete = delete_len > 0;
        let has_insert = !self.insert.is_empty();

        let mut ops = Vec::new();

        if has_delete {
            ops.push(deve_core::models::Op::Delete {
                pos: self.from,
                len: delete_len,
            });
        }

        if has_insert {
            ops.push(deve_core::models::Op::Insert {
                pos: self.from, // 插入位置 = 删除位置
                content: self.insert.clone(),
            });
        }

        ops
    }
}
