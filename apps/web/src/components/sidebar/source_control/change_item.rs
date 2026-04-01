// apps\web\src\components\sidebar\source_control
//! # ChangeItem 组件 (变更条目组件)
//!
//! 渲染单个文件变更条目，包含文件图标、名称、路径和状态标记。
//! 支持 Stage/Unstage/Open/Discard 操作。

use crate::components::icons::*;
use crate::components::sidebar::source_control::change_item_actions::ChangeItemActions;
use crate::components::sidebar::source_control::change_item_counterpart::{
    counterpart_badge_text, counterpart_badge_title, find_counterpart_kind,
};
use crate::components::sidebar::source_control::change_item_meta::build_change_item_meta;
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
    let write_block = core.write_block;
    let action_busy = StoredValue::new(Arc::new(AtomicBool::new(false)));

    let has_conflict = entry.has_conflict;
    let can_open_diff = can_request_doc_diff(&entry);
    let meta = build_change_item_meta(&entry);
    let entry_for_click = entry.clone();
    let entry_for_counterpart = entry.clone();

    let action_busy_reset = action_busy;
    Effect::new(move |_| {
        let _ = core.staged_changes.get();
        let _ = core.unstaged_changes.get();
        let _ = core.notice.get();
        action_busy_reset
            .get_value()
            .store(false, Ordering::Release);
    });

    view! {
        <div
            class=format!(
                "flex items-center px-4 py-0.5 hover:bg-hover text-[13px] group h-[22px] {} {}",
                if has_conflict { "text-warning bg-warning/5" } else { "text-primary" },
                if can_open_diff { "cursor-pointer" } else { "cursor-help" }
            )
            on:click=move |_| {
                if current_repo_id.get().is_none()
                    || pending_branch_switch.get().is_some()
                    || pending_repo_switch.get().is_some()
                    || write_block.get().is_some()
                {
                    return;
                }
                // Diff 不可用的条目会在回调里写入明确的 Source Control notice。
                core.on_get_doc_diff.run(entry_for_click.clone());
            }
        >
            <div class="flex items-center gap-1.5 flex-1 overflow-hidden">
                <FileText class=format!("w-3.5 h-3.5 min-w-3.5 {}", meta.file_icon_class) />

                <span class="truncate">{meta.display_name}</span>
                <span class="text-xs text-muted truncate shrink-0 ml-1">
                    {meta.directory}
                </span>
                {move || {
                    let counterpart = find_counterpart_kind(
                        &entry_for_counterpart,
                        is_staged,
                        &core.staged_changes.get(),
                        &core.unstaged_changes.get(),
                    );
                    counterpart.map(|kind| {
                        let locale_value = locale.get();
                        view! {
                            <span
                                class="ml-1 shrink-0 rounded border border-border px-1 py-px text-[10px] font-semibold text-muted"
                                title=counterpart_badge_title(kind, locale_value)
                            >
                                {counterpart_badge_text(kind, locale_value)}
                            </span>
                        }
                    })
                }}
            </div>

            <div class="flex items-center gap-2 pl-2">
                // 操作按钮 (悬停显示)
                <div class="hidden group-hover:!flex items-center gap-0.5 mr-1">
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

                // 冲突指示 + 状态标记
                {if has_conflict {
                    view! { <AlertTriangle class="w-3 h-3 text-warning mr-0.5" /> }.into_any()
                } else {
                    view! {}.into_any()
                }}
                <span class=format!("{} text-[11px] font-bold w-3 text-center", meta.color_class)>
                    {meta.icon_char}
                </span>
            </div>
        </div>
    }
}
