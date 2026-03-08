use std::cell::Cell;

thread_local! {
    static NEXT_CLIENT_OP_ID: Cell<u64> = const { Cell::new(1) };
}

/// 生成当前浏览器会话内单调递增的编辑操作 ID。
pub fn next_client_op_id() -> u64 {
    NEXT_CLIENT_OP_ID.with(|cell| {
        let id = cell.get();
        cell.set(id.saturating_add(1));
        id
    })
}
