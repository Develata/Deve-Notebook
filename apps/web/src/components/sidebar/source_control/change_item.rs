// apps\web\src\components\sidebar\source_control
//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 09_web_thin_client_ledger#web-edit-intent
//!
//! # ChangeItem 组件 (变更条目组件)
//!
//! 渲染单个文件变更条目，包含文件图标、名称、路径和状态标记。
//! 支持 Stage/Unstage/Open/Discard 操作。

use crate::components::sidebar::source_control::action_tray::CHANGE_ITEM_ACTION_TRAY_CLASS;
use crate::components::sidebar::source_control::change_item_actions::ChangeItemActions;
use crate::components::sidebar::source_control::change_item_content::ChangeItemContent;
use crate::components::sidebar::source_control::change_item_meta::build_change_item_meta;
use crate::components::sidebar::source_control::change_item_read_gate::can_open_change_item_diff;
use crate::components::sidebar::source_control::touch_target::change_item_row_class;
use crate::hooks::use_core::{SourceControlContext, can_request_doc_diff};
use crate::i18n::Locale;
use deve_core::source_control::ChangeEntry;
use leptos::prelude::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// 变更条目组件
///
/// # 参数
/// - `entry`: 变更条目数据
/// - `is_staged`: 是否为暂存区条目
#[component]
pub fn ChangeItem(entry: ChangeEntry, is_staged: bool) -> impl IntoView {
    let core = expect_context::<SourceControlContext>();
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));
    let current_repo_id = core.current_repo_id;
    let pending_branch_switch = core.pending_branch_switch;
    let pending_repo_switch = core.pending_repo_switch;
    let read_block = core.read_block;
    let action_busy = StoredValue::new(Arc::new(AtomicBool::new(false)));

    let has_conflict = entry.has_conflict;
    let can_open_diff = can_request_doc_diff(&entry);
    let meta = build_change_item_meta(&entry);
    let entry_for_click = entry.clone();

    let action_busy_reset = action_busy;
    Effect::new(move |_| {
        let _ = core.staged_changes.get();
        let _ = core.unstaged_changes.get();
        let _ = core.confirmed_changes.get();
        action_busy_reset
            .get_value()
            .store(false, Ordering::Release);
    });

    let action_busy_feedback_reset = action_busy;
    Effect::new(move |_| {
        let notice = core.notice.get();
        let write_block = core.write_block.get();
        if notice.is_some() || write_block.is_some() {
            action_busy_feedback_reset
                .get_value()
                .store(false, Ordering::Release);
        }
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
                is_staged
                meta
                has_conflict
                staged_changes=core.staged_changes
                unstaged_changes=core.unstaged_changes
            />

            <div class="flex items-center gap-2 pl-2">
                // 行操作区在桌面和触屏环境都保持可达，避免 hover-only 交互。
                <div
                    class=CHANGE_ITEM_ACTION_TRAY_CLASS
                    data-deve-sc-action-tray="row"
                >
                    <ChangeItemActions
                        core=core.clone()
                        locale
                        entry=entry.clone()
                        is_staged
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
