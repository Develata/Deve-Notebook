//! plan_ref:
//!   - 13_i18n#i18n-keys-reference
//!   - 09_web_thin_client_ledger#write-readiness

use crate::i18n::Locale;

use super::{WriteGateAction, WriteGateReason, action_label, reason_label};

pub fn cannot_action(locale: Locale, action: WriteGateAction, reason: WriteGateReason) -> String {
    match locale {
        Locale::En => format!(
            "Cannot {}: {}",
            action_label(locale, action),
            reason_label(locale, reason)
        ),
        Locale::Zh => format!(
            "无法{}：{}",
            action_label(locale, action),
            reason_label(locale, reason)
        ),
    }
}

pub fn cannot_send(locale: Locale, action: WriteGateAction, reason: WriteGateReason) -> String {
    match locale {
        Locale::En => format!(
            "Cannot send {}: {}",
            action_label(locale, action),
            reason_label(locale, reason)
        ),
        Locale::Zh => format!(
            "无法发送 {}：{}",
            action_label(locale, action),
            reason_label(locale, reason)
        ),
    }
}
