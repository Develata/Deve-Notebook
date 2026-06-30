//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 09_web_thin_client_ledger#web-edit-intent
//!
use crate::hooks::use_core::write_gate::RepoWriteBlock;
pub(crate) use crate::i18n::write_gate::{WriteGateAction, WriteGateReason};
use crate::i18n::{Locale, t};

pub(crate) fn reason_from_block(block: RepoWriteBlock) -> WriteGateReason {
    match block {
        RepoWriteBlock::SessionExpired => WriteGateReason::SessionExpired,
        RepoWriteBlock::NativeBootstrapInvalid => WriteGateReason::NativeBootstrapInvalid,
        RepoWriteBlock::NativeSessionPending => WriteGateReason::NativeSessionPending,
        RepoWriteBlock::NativeServiceOffline => WriteGateReason::NativeServiceOffline,
        RepoWriteBlock::NativeReprobeRequired => WriteGateReason::NativeReprobeRequired,
        RepoWriteBlock::Offline => WriteGateReason::Offline,
        RepoWriteBlock::Reconnecting => WriteGateReason::Reconnecting,
        RepoWriteBlock::SnapshotLoading => WriteGateReason::SnapshotLoading,
        RepoWriteBlock::ReadOnly => WriteGateReason::ReadOnly,
        RepoWriteBlock::ScopeSwitching => WriteGateReason::ScopeSwitching,
        RepoWriteBlock::NoRepo => WriteGateReason::NoRepo,
        RepoWriteBlock::HandshakingRepo => WriteGateReason::HandshakingRepo,
    }
}

pub(crate) fn cannot_action(
    locale: Locale,
    action: WriteGateAction,
    reason: WriteGateReason,
) -> String {
    t::write_gate::cannot_action(locale, action, reason)
}

pub(crate) fn cannot_send(
    locale: Locale,
    action: WriteGateAction,
    reason: WriteGateReason,
) -> String {
    t::write_gate::cannot_send(locale, action, reason)
}

pub(crate) fn cannot_create_document(locale: Locale, reason: WriteGateReason) -> String {
    cannot_action(locale, WriteGateAction::CreateDocument, reason)
}

#[cfg(test)]
mod tests {
    use super::{
        WriteGateAction, WriteGateReason, cannot_action, cannot_create_document, cannot_send,
        reason_from_block,
    };
    use crate::hooks::use_core::write_gate::RepoWriteBlock;
    use crate::i18n::Locale;

    #[test]
    fn write_gate_banner_uses_i18n_copy() {
        assert_eq!(
            cannot_action(
                Locale::En,
                WriteGateAction::MoveDocument,
                WriteGateReason::ReadOnly
            ),
            "Cannot move document: read-only"
        );
        assert_eq!(
            cannot_send(
                Locale::Zh,
                WriteGateAction::DeleteDoc,
                WriteGateReason::Offline
            ),
            "无法发送 删除文档请求：离线"
        );
        assert_eq!(
            cannot_create_document(Locale::Zh, WriteGateReason::SnapshotLoading),
            "无法新建文档：正在加载快照"
        );
        assert_eq!(
            reason_from_block(RepoWriteBlock::HandshakingRepo),
            WriteGateReason::HandshakingRepo
        );
    }
}
