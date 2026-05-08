//! Pending navigation copy.
//! plan_ref:
//!   - 11_i18n#i18n-keys-reference

use super::Locale;

pub fn pending_navigation_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Unconfirmed Local Edits",
        Locale::Zh => "存在未确认本地写入",
    }
}

pub fn pending_navigation_body(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "The current document still has local edits waiting for server acknowledgement."
        }
        Locale::Zh => "当前文档仍有等待服务端确认的本地写入。",
    }
}

pub fn pending_navigation_note(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "Continuing only leaves this view. It does not mean those edits were committed."
        }
        Locale::Zh => "继续只会离开当前视图，不代表这些写入已经提交。",
    }
}

pub fn pending_navigation_destination(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Destination",
        Locale::Zh => "目标",
    }
}

pub fn pending_navigation_doc(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Another document",
        Locale::Zh => "另一篇文档",
    }
}

pub fn pending_navigation_repo(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Another repository",
        Locale::Zh => "另一个仓库",
    }
}

pub fn pending_navigation_branch(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Another branch",
        Locale::Zh => "另一个分支",
    }
}

pub fn pending_navigation_home(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Home",
        Locale::Zh => "首页",
    }
}

pub fn pending_navigation_cancel(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Cancel",
        Locale::Zh => "取消",
    }
}

pub fn pending_navigation_continue(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Leave View",
        Locale::Zh => "继续切换",
    }
}
