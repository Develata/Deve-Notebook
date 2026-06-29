//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!   - 09_web_thin_client_ledger#web-edit-intent
//!
use crate::api::WsService;
use crate::hooks::use_core::callbacks_scope::{LocalScopeSignals, stable_local_scope_nonce};
use crate::hooks::use_core::sync_banner_notice::warn_sync_banner;
use crate::hooks::use_core::write_gate::{RepoWriteSignals, repo_write_block_untracked};
use crate::hooks::use_core::write_gate_banner::{
    WriteGateAction, WriteGateReason, cannot_send, reason_from_block,
};
use crate::i18n::Locale;
use leptos::prelude::{GetUntracked, RwSignal, WriteSignal};

pub(super) fn local_write_scope_nonce(
    ws: &WsService,
    locale: RwSignal<Locale>,
    local_scope: LocalScopeSignals,
    write_gate: RepoWriteSignals,
    set_sync_banner: WriteSignal<Option<String>>,
    action: WriteGateAction,
) -> Option<u64> {
    if let Some(block) = repo_write_block_untracked(ws, write_gate) {
        show_doc_write_block(set_sync_banner, locale, action, reason_from_block(block));
        return None;
    }
    let Some(scope_nonce) = stable_local_scope_nonce(local_scope) else {
        show_doc_write_block(
            set_sync_banner,
            locale,
            action,
            WriteGateReason::LocalRepoScopeUnstable,
        );
        return None;
    };
    Some(scope_nonce)
}

fn show_doc_write_block(
    set_sync_banner: WriteSignal<Option<String>>,
    locale: RwSignal<Locale>,
    action: WriteGateAction,
    reason: WriteGateReason,
) {
    let message = cannot_send(locale.get_untracked(), action, reason);
    warn_sync_banner(set_sync_banner, message);
}

#[cfg(test)]
mod tests {
    use super::show_doc_write_block;
    use crate::hooks::use_core::write_gate_banner::{WriteGateAction, WriteGateReason};
    use crate::i18n::Locale;
    use leptos::prelude::{GetUntracked, signal};

    #[test]
    fn doc_write_block_banner_uses_i18n_copy() {
        let (sync_banner, set_sync_banner) = signal(None::<String>);
        let locale = leptos::prelude::RwSignal::new(Locale::Zh);

        show_doc_write_block(
            set_sync_banner,
            locale,
            WriteGateAction::MoveDoc,
            WriteGateReason::LocalRepoScopeUnstable,
        );

        assert_eq!(
            sync_banner.get_untracked().as_deref(),
            Some("无法发送 移动文档请求：本地仓库作用域不稳定")
        );
    }
}
