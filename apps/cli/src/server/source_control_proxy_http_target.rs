//! plan_ref:
//!   - 07_diff_logic#source-control-runtime

use deve_core::protocol::{ServerError, ServerErrorCode};

pub(crate) enum ProxyScOp {
    StagePending(String),
    DiscardPending(String),
    Unstage(String),
    DiffDoc(String),
}

/// target-aware 远端 source-control 错误映射。
///
/// # 不变量
/// - 同一 target 文案在 remote proxy 上必须与本地 `ScOp` 映射保持同口径。
/// - 仅在调用方显式提供 target operation 时才允许走这层细分映射。
pub(super) fn classify_op_specific_error(
    op: Option<&ProxyScOp>,
    lower: &str,
) -> Option<ServerError> {
    let op = op?;
    match op {
        ProxyScOp::StagePending(path) | ProxyScOp::DiscardPending(path)
            if contains_any(
                lower,
                &[
                    "ambiguous pending_fs target",
                    "tracked source control target requires document identity",
                ],
            ) =>
        {
            Some(ServerError::with_detail(
                ServerErrorCode::StorageConflict,
                path.clone(),
            ))
        }
        ProxyScOp::StagePending(path) | ProxyScOp::DiscardPending(path)
            if contains_any(
                lower,
                &[
                    "path is not in pending_fs_ops",
                    "source control target not resolved for path",
                    "source control target not resolved for doc",
                ],
            ) =>
        {
            Some(ServerError::with_detail(
                ServerErrorCode::ScPendingNotFound,
                path.clone(),
            ))
        }
        ProxyScOp::Unstage(path)
            if contains_any(
                lower,
                &[
                    "ambiguous staged target",
                    "tracked source control target requires document identity",
                ],
            ) =>
        {
            Some(ServerError::with_detail(
                ServerErrorCode::StorageConflict,
                path.clone(),
            ))
        }
        ProxyScOp::Unstage(path)
            if contains_any(
                lower,
                &[
                    "path is not staged",
                    "source control target not resolved for path",
                    "source control target not resolved for doc",
                ],
            ) =>
        {
            Some(ServerError::with_detail(
                ServerErrorCode::ScStagedNotFound,
                path.clone(),
            ))
        }
        ProxyScOp::DiffDoc(path)
            if contains_any(
                lower,
                &[
                    "ambiguous pending_fs target",
                    "ambiguous staged target",
                    "tracked source control target requires document identity",
                ],
            ) =>
        {
            Some(ServerError::with_detail(
                ServerErrorCode::StorageConflict,
                path.clone(),
            ))
        }
        ProxyScOp::DiffDoc(path)
            if contains_any(
                lower,
                &[
                    "doc not found",
                    "document not found",
                    "remote document not found",
                    "source control target not resolved for path",
                    "source control target not resolved for doc",
                ],
            ) =>
        {
            Some(ServerError::with_detail(
                ServerErrorCode::ScDocNotFound,
                path.clone(),
            ))
        }
        _ => None,
    }
}

fn contains_any(input: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| input.contains(pattern))
}
