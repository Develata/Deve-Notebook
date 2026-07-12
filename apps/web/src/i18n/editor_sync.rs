//! plan_ref:
//!   - 10_rendering#document-authority-bridge

use super::Locale;
use crate::runtime::domain::EditorSyncFailureCode;

pub fn title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Editor synchronization stopped",
        Locale::Zh => "编辑器同步已停止",
    }
}

pub fn detail(locale: Locale, code: EditorSyncFailureCode) -> &'static str {
    match (locale, code) {
        (Locale::En, EditorSyncFailureCode::SnapshotApply) => {
            "The snapshot could not be applied after one automatic reopen."
        }
        (Locale::Zh, EditorSyncFailureCode::SnapshotApply) => {
            "快照在一次自动重新打开后仍无法应用。"
        }
        (Locale::En, EditorSyncFailureCode::DeltaReplay) => {
            "A confirmed delta batch could not be applied atomically."
        }
        (Locale::Zh, EditorSyncFailureCode::DeltaReplay) => "已确认的增量批次无法原子应用。",
        (Locale::En, EditorSyncFailureCode::HistoryReplay) => {
            "Confirmed history could not be replayed atomically."
        }
        (Locale::Zh, EditorSyncFailureCode::HistoryReplay) => "已确认的历史无法原子重放。",
        (Locale::En, EditorSyncFailureCode::LiveReplay) => {
            "A live confirmed edit could not be applied."
        }
        (Locale::Zh, EditorSyncFailureCode::LiveReplay) => "实时确认编辑无法应用。",
        (Locale::En, EditorSyncFailureCode::ContentReadback) => {
            "The editor projection could not be read back safely."
        }
        (Locale::Zh, EditorSyncFailureCode::ContentReadback) => "无法安全读取编辑器投影。",
    }
}

pub fn retry(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Retry document sync",
        Locale::Zh => "重试文档同步",
    }
}
