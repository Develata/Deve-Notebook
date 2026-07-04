//! plan_ref:
//!   - 13_i18n#i18n-keys-reference
//!   - 14_commands#command-palette-shortcuts

use crate::i18n::Locale;

pub fn switch_peer(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "P2P: Switch to Peer",
        Locale::Zh => "P2P: 切换到节点",
    }
}

pub fn establish_branch(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "P2P: Establish Branch",
        Locale::Zh => "P2P: 建立分支",
    }
}

pub fn establish_branch_unavailable_reason(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Unavailable: no branch creation backend",
        Locale::Zh => "不可用：尚无分支创建后端",
    }
}

pub fn merge_peer(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "P2P: Merge Peer",
        Locale::Zh => "P2P: 合并节点",
    }
}

pub fn merge_peer_no_source_reason(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Unavailable: no peer mirror is available for the current local branch",
        Locale::Zh => "不可用：当前本地分支没有可合并的 peer mirror",
    }
}

pub fn merge_peer_context_unavailable_reason(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Unavailable: merge peer requires local branch and sync context",
        Locale::Zh => "不可用：合并 peer 需要本地分支与同步上下文",
    }
}
