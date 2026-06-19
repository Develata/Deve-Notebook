//! Source Control gate and notice copy.
//! plan_ref:
//!   - 13_i18n#i18n-keys-reference

use super::Locale;

pub fn no_repo_selected(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "No repo selected",
        Locale::Zh => "尚未选择仓库",
    }
}

pub fn scope_switching(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Switching scope...",
        Locale::Zh => "切换作用域中...",
    }
}

pub fn session_expired_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Sign in again before staging, discarding, or committing changes.",
        Locale::Zh => "请重新登录后再暂存、放弃或提交更改。",
    }
}

pub fn offline_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Wait for the connection to recover before changing Source Control state.",
        Locale::Zh => "请等待连接恢复后再修改源代码管理状态。",
    }
}

pub fn reconnecting_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "The client is reconnecting. Source Control actions will resume automatically."
        }
        Locale::Zh => "客户端正在重连，源代码管理操作会在恢复后自动可用。",
    }
}

pub fn snapshot_loading_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Wait for the current repo snapshot to finish loading.",
        Locale::Zh => "请等待当前仓库快照加载完成。",
    }
}

pub fn scope_switching_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Wait for the repo or branch switch to finish before editing changes.",
        Locale::Zh => "请等待仓库或分支切换完成后再修改更改列表。",
    }
}

pub fn no_repo_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Select an active repo before using Source Control actions.",
        Locale::Zh => "请先选择激活仓库，再使用源代码管理操作。",
    }
}

pub fn handshaking_repo_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "This repo is still negotiating writer access. Try again in a moment.",
        Locale::Zh => "当前仓库仍在协商写入权限，请稍后再试。",
    }
}

pub fn diff_unavailable(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Diff unavailable",
        Locale::Zh => "无法显示差异",
    }
}

pub fn deleted_change_no_doc_diff(locale: Locale, path: &str) -> String {
    match locale {
        Locale::En => format!(
            "No diff is available for deleted change {path} because it has no document identity."
        ),
        Locale::Zh => format!("删除变更 {path} 没有文档身份，因此当前无法生成可显示的差异。"),
    }
}

pub fn legacy_commit_unprojectable(locale: Locale, commit: Option<&str>) -> String {
    match (locale, commit) {
        (Locale::En, Some(commit)) => format!(
            "Commit {commit} contains legacy content without structure projection, so Deve-Note cannot reconstruct a path-safe diff."
        ),
        (Locale::Zh, Some(commit)) => format!(
            "提交 {commit} 包含缺少结构投影的旧内容，Deve-Note 无法安全重建带路径语义的差异。"
        ),
        (Locale::En, None) => {
            "This legacy commit contains content without structure projection, so Deve-Note cannot reconstruct a path-safe diff.".to_string()
        }
        (Locale::Zh, None) => {
            "该旧提交包含缺少结构投影的内容，Deve-Note 无法安全重建带路径语义的差异。".to_string()
        }
    }
}

pub fn stage_files_before_commit(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Stage files before trying to commit.",
        Locale::Zh => "请先暂存文件，再执行提交。",
    }
}

pub fn refresh_change_list(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Refresh the change list and try again.",
        Locale::Zh => "请刷新更改列表后再试。",
    }
}

pub fn selected_item_unavailable(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "The selected Source Control item is no longer available.",
        Locale::Zh => "当前选中的源代码管理条目已不存在。",
    }
}

pub fn establish_branch_unavailable_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "P2P branch creation is unavailable",
        Locale::Zh => "P2P 分支创建不可用",
    }
}

pub fn establish_branch_unavailable_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => {
            "Current Web runtime can switch to peer branches for read-only inspection and run P2P: Merge Peer from the local branch; creating a new local branch from a peer branch has no backend contract yet."
        }
        Locale::Zh => {
            "当前 Web runtime 可以切换到 peer 分支做只读查看，并从本地分支执行 P2P: Merge Peer；从 peer 分支创建本地分支尚无后端合同。"
        }
    }
}

pub fn loading_commit_diff(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Loading commit diff...",
        Locale::Zh => "正在加载提交差异...",
    }
}

pub fn counterpart_staged_badge(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "IDX",
        Locale::Zh => "暂存区",
    }
}

pub fn counterpart_working_tree_badge(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "WT",
        Locale::Zh => "工作区",
    }
}

pub fn counterpart_staged_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Also present in Staged Changes",
        Locale::Zh => "对应改动也存在于暂存区",
    }
}

pub fn counterpart_working_tree_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Also modified in Working Directory",
        Locale::Zh => "对应改动也存在于工作区",
    }
}
