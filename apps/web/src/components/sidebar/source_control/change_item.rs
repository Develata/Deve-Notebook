// apps\web\src\components\sidebar\source_control
//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 09_web_thin_client_ledger#web-edit-intent
//!
//! # ChangeItem 组件 (变更条目组件)
//!
//! 渲染单个文件变更条目，包含文件图标、名称、路径和状态标记。
//! 支持 Stage/Unstage/Open/Discard 操作。

use crate::components::sidebar::source_control::change_item_actions::ChangeItemActions;
use crate::components::sidebar::source_control::change_item_content::ChangeItemContent;
use crate::components::sidebar::source_control::change_item_meta::build_change_item_meta;
use crate::components::sidebar::source_control::change_item_read_gate::can_open_change_item_diff;
use crate::components::sidebar::source_control::touch_target::{
    change_item_action_container_class, change_item_row_class,
};
use crate::hooks::use_core::{SourceControlContext, can_request_doc_diff};
use crate::i18n::Locale;
use deve_core::source_control::{ChangeDomain, ChangeEntry};
use leptos::prelude::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ChangeItemKind {
    Working,
    Staged,
    ConfirmedLedger,
}

impl ChangeItemKind {
    pub(crate) const fn is_staged(self) -> bool {
        matches!(self, Self::Staged)
    }

    pub(crate) const fn is_confirmed_ledger(self) -> bool {
        matches!(self, Self::ConfirmedLedger)
    }

    pub(crate) const fn shows_counterpart_badge(self) -> bool {
        matches!(self, Self::Working | Self::Staged)
    }
}

pub(crate) fn effective_change_item_kind(
    row_kind: ChangeItemKind,
    entry: &ChangeEntry,
) -> ChangeItemKind {
    if entry.domain == ChangeDomain::ConfirmedLedger {
        ChangeItemKind::ConfirmedLedger
    } else {
        row_kind
    }
}

/// 变更条目组件
///
/// # 参数
/// - `entry`: 变更条目数据
/// - `row_kind`: 行所在 Source Control 分组语义
#[component]
pub fn ChangeItem(entry: ChangeEntry, row_kind: ChangeItemKind) -> impl IntoView {
    let core = expect_context::<SourceControlContext>();
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));
    let current_repo_id = core.current_repo_id;
    let pending_branch_switch = core.pending_branch_switch;
    let pending_repo_switch = core.pending_repo_switch;
    let read_block = core.read_block;
    let action_busy = StoredValue::new(Arc::new(AtomicBool::new(false)));

    let has_conflict = entry.has_conflict;
    let can_open_diff = can_request_doc_diff(&entry);
    let row_kind = effective_change_item_kind(row_kind, &entry);
    let meta = build_change_item_meta(&entry);
    let entry_for_click = entry.clone();

    let action_busy_reset = action_busy;
    Effect::new(move |_| {
        let _ = core.staged_changes.get();
        let _ = core.unstaged_changes.get();
        let _ = core.confirmed_changes.get();
        let _ = core.notice.get();
        action_busy_reset
            .get_value()
            .store(false, Ordering::Release);
    });

    view! {
        <div
            class=change_item_row_class(has_conflict, can_open_diff)
            data-deve-mobile-touch-target="source-control-change-row"
            on:click=move |_| {
                if !can_open_change_item_diff(
                    current_repo_id.get().is_some(),
                    pending_branch_switch.get().is_some(),
                    pending_repo_switch.get().is_some(),
                    read_block.get().is_some(),
                ) {
                    return;
                }
                // Diff 不可用的条目会在回调里写入明确的 Source Control notice。
                core.on_get_doc_diff.run(entry_for_click.clone());
            }
        >
            <ChangeItemContent
                locale
                entry=entry.clone()
                row_kind
                meta
                has_conflict
                staged_changes=core.staged_changes
                unstaged_changes=core.unstaged_changes
            />

            <div class="flex items-center gap-2 pl-2">
                // Confirmed ledger 只有打开 diff 一个合法行操作，必须常驻可见。
                <div class=change_item_action_container_class(row_kind.is_confirmed_ledger())>
                    <ChangeItemActions
                        core=core.clone()
                        locale
                        entry=entry.clone()
                        row_kind
                        has_conflict
                        can_open_diff
                        action_busy
                    />
                </div>

                // 右侧 hover 操作区与主内容分离，避免挤压文件名。
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::{ChangeItemKind, effective_change_item_kind};
    use deve_core::source_control::{ChangeDomain, ChangeEntry, ChangeStatus};

    fn entry(domain: ChangeDomain) -> ChangeEntry {
        ChangeEntry {
            path: "note.md".into(),
            renamed_from: None,
            doc_id: None,
            status: ChangeStatus::Modified,
            has_conflict: false,
            domain,
            base_seq: None,
            target_seq: None,
        }
    }

    #[test]
    fn confirmed_ledger_domain_forces_confirmed_row_kind() {
        assert_eq!(
            effective_change_item_kind(
                ChangeItemKind::Working,
                &entry(ChangeDomain::ConfirmedLedger)
            ),
            ChangeItemKind::ConfirmedLedger
        );
    }
}
