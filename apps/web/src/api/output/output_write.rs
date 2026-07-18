//! plan_ref:
//!   - 08_auth#unauthorized-handling
//!   - 08_auth#unauthorized-disconnected-ui
//!

use deve_core::protocol::ClientMessage;
use std::collections::VecDeque;

pub(crate) fn drop_queued_writes(queue: &mut VecDeque<ClientMessage>) {
    queue.retain(|msg| !is_write_message(msg));
}

/// 判断消息是否为写入类操作
///
/// WebLightPeer 约束：断连时禁止写入，只允许查询类消息。
pub(crate) fn is_write_message(msg: &ClientMessage) -> bool {
    matches!(
        msg,
        ClientMessage::Edit { .. }
            | ClientMessage::CreateDoc { .. }
            | ClientMessage::RenameDoc { .. }
            | ClientMessage::DeleteDoc { .. }
            | ClientMessage::CopyDoc { .. }
            | ClientMessage::MoveDoc { .. }
            | ClientMessage::RegisterWriter { .. }
            | ClientMessage::SyncPush { .. }
            | ClientMessage::SyncPushSnapshot { .. }
            | ClientMessage::Commit { .. }
            | ClientMessage::ApplyExternalChanges { .. }
            | ClientMessage::StageFile { .. }
            | ClientMessage::StageFiles { .. }
            | ClientMessage::UnstageFile { .. }
            | ClientMessage::UnstageFiles { .. }
            | ClientMessage::DiscardFile { .. }
            | ClientMessage::ResolveConflict { .. }
            | ClientMessage::ResolveMergeConflict { .. }
            | ClientMessage::MergePeer { .. }
            | ClientMessage::RemoteProjectionPush(_)
            | ClientMessage::DeletePeer { .. }
            | ClientMessage::SwitchBranch { .. }
            | ClientMessage::SwitchRepo { .. }
            | ClientMessage::SwitchRepoExact { .. }
            | ClientMessage::CreateRepo { .. }
            | ClientMessage::RenameRepo { .. }
            | ClientMessage::RemoveRepo { .. }
            | ClientMessage::ConfirmMerge { .. }
            | ClientMessage::DiscardPending { .. }
            | ClientMessage::SetSyncMode { .. }
            | ClientMessage::PluginCall { .. }
    ) || matches!(msg, ClientMessage::RemoteImport(request) if request.is_write())
}
