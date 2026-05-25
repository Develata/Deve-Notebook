//! plan_ref:
//!   - 13_i18n#i18n-keys-reference
//!
//! # I18n Dashboard Module (仪表盘翻译)
//!
//! 包含服务器仪表盘相关的翻译字符串。

use super::Locale;

pub fn title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Server Dashboard",
        Locale::Zh => "服务器仪表盘",
    }
}

pub fn waiting_metrics(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Waiting for server metrics...",
        Locale::Zh => "等待服务器指标...",
    }
}

pub fn metrics_live(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Metrics live via WebSocket.",
        Locale::Zh => "指标正通过 WebSocket 实时更新。",
    }
}

pub fn metrics_disconnected(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Disconnected; showing last metrics snapshot.",
        Locale::Zh => "已断开连接；显示最后一次指标快照。",
    }
}

pub fn no_repo_selected(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "No repo selected",
        Locale::Zh => "尚未选择仓库",
    }
}

pub fn docs_in_current_repo(locale: Locale, count: usize) -> String {
    match locale {
        Locale::En => format!("{count} docs in current repo"),
        Locale::Zh => format!("当前仓库 {count} 篇文档"),
    }
}

pub fn quick_actions(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Quick Actions",
        Locale::Zh => "快捷操作",
    }
}

pub fn new_doc(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "New Doc",
        Locale::Zh => "新建文档",
    }
}

pub fn sync_now(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Sync Now",
        Locale::Zh => "立即同步",
    }
}

pub fn storage(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Storage",
        Locale::Zh => "存储",
    }
}

pub fn db_size(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "DB Size",
        Locale::Zh => "数据库大小",
    }
}

pub fn documents(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Documents",
        Locale::Zh => "文档数",
    }
}

pub fn server_health(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Server Health",
        Locale::Zh => "服务器健康",
    }
}

pub fn runtime_info(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Runtime Info",
        Locale::Zh => "运行信息",
    }
}

pub fn runtime_shape(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Shape",
        Locale::Zh => "形态",
    }
}

pub fn runtime_waiting(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Waiting for runtime info...",
        Locale::Zh => "等待运行信息...",
    }
}

pub fn cpu(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "CPU",
        Locale::Zh => "CPU",
    }
}

pub fn memory(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Memory",
        Locale::Zh => "内存",
    }
}

pub fn uptime(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Uptime",
        Locale::Zh => "运行时间",
    }
}

pub fn sync_status(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Sync Status",
        Locale::Zh => "同步状态",
    }
}

pub fn connected_peers(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Connected Peers",
        Locale::Zh => "已连接节点",
    }
}

pub fn ops_processed(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Ops Processed",
        Locale::Zh => "已处理操作",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dashboard_repo_summary_is_localized() {
        assert_eq!(no_repo_selected(Locale::En), "No repo selected");
        assert_eq!(no_repo_selected(Locale::Zh), "尚未选择仓库");
        assert_eq!(
            docs_in_current_repo(Locale::En, 3),
            "3 docs in current repo"
        );
        assert_eq!(docs_in_current_repo(Locale::Zh, 3), "当前仓库 3 篇文档");
    }
}
