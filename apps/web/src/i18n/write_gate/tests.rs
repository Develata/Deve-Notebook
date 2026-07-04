use super::*;
use crate::i18n::Locale;

#[test]
fn write_gate_banner_copy_is_localized() {
    assert_eq!(
        cannot_action(
            Locale::En,
            WriteGateAction::Search,
            WriteGateReason::SnapshotLoading
        ),
        "Cannot search: snapshot loading"
    );
    assert_eq!(
        cannot_action(
            Locale::Zh,
            WriteGateAction::Search,
            WriteGateReason::SnapshotLoading
        ),
        "无法搜索：正在加载快照"
    );
    assert_eq!(
        cannot_send(
            Locale::Zh,
            WriteGateAction::Commit,
            WriteGateReason::HandshakingRepo
        ),
        "无法发送 提交请求：正在协商仓库写入权限"
    );
    assert_eq!(
        reason_label(Locale::Zh, WriteGateReason::FileTooLarge),
        "文件超过 1 MiB"
    );
}
