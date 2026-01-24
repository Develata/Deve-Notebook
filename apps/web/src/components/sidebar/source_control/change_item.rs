// apps\web\src\components\sidebar\source_control
//! # ChangeItem 组件 (变更条目组件)
//!
//! 渲染单个文件变更条目，包含文件图标、名称、路径和状态标记。
//! 支持 Stage/Unstage/Open/Discard 操作。

use crate::hooks::use_core::CoreState;
use deve_core::source_control::{ChangeEntry, ChangeStatus};
use leptos::prelude::*;

/// 变更条目组件
///
/// # 参数
/// - `entry`: 变更条目数据
/// - `is_staged`: 是否为暂存区条目
#[component]
pub fn ChangeItem(entry: ChangeEntry, is_staged: bool) -> impl IntoView {
    let core = expect_context::<CoreState>();

    let full_path = entry.path.clone();
    let path_parts: Vec<&str> = full_path.split('/').collect();
    let filename = path_parts.last().unwrap_or(&"?").to_string();

    // 目录路径 (不含文件名)
    let directory = if path_parts.len() > 1 {
        path_parts[..path_parts.len() - 1].join("/")
    } else {
        String::new()
    };

    let path_for_stage = full_path.clone();
    let path_for_unstage = full_path.clone();
    let path_for_open = full_path.clone();
    let path_for_discard = full_path.clone();

    // 状态图标和颜色
    let (icon_char, color_cls) = match entry.status {
        ChangeStatus::Modified => ("M", "text-[#d7ba7d]"),
        ChangeStatus::Added => ("A", "text-[#73c991]"),
        ChangeStatus::Deleted => ("D", "text-[#f14c4c]"),
    };

    view! {
        <div
            class="flex items-center px-4 py-0.5 hover:bg-[#eff1f3] dark:hover:bg-[#37373d] text-[13px] group cursor-pointer h-[22px] text-[#333] dark:text-[#cccccc]"
            on:click=move |_| {
                // 点击任何条目都打开 diff 视图 (与 VS Code 行为一致)
                core.on_get_doc_diff.run(full_path.clone());
            }
        >
            <div class="flex items-center gap-1.5 flex-1 overflow-hidden">
                // 文件图标
                <svg xmlns="http://www.w3.org/2000/svg" class=format!("w-3.5 h-3.5 min-w-3.5 {}", if filename.ends_with(".rs") { "text-[#dea584]" } else { "text-gray-400" }) viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z"/><polyline points="14 2 14 8 20 8"/></svg>

                <span class="truncate">{filename}</span>
                <span class="text-xs text-[#808080] truncate shrink-0 ml-1">
                    {directory}
                </span>
            </div>

            <div class="flex items-center gap-2 pl-2">
                // 操作按钮 (悬停显示)
                <div class="hidden group-hover:!flex items-center gap-0.5 mr-1">
                    {if is_staged {
                        // 暂存区: 仅显示 Unstage 按钮
                        view! {
                            <button
                                class="p-0.5 hover:bg-[#d0d0d0] dark:hover:bg-[#454545] rounded text-gray-600 dark:text-gray-300"
                                title="Unstage Changes"
                                on:click=move |ev| { ev.stop_propagation(); core.on_unstage_file.run(path_for_unstage.clone()); }
                            >
                                <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="5" y1="12" x2="19" y2="12"/></svg>
                            </button>
                        }.into_any()
                    } else {
                        // 工作区: Open, Discard, Stage
                        view! {
                            <button
                                class="p-0.5 hover:bg-[#d0d0d0] dark:hover:bg-[#454545] rounded text-gray-600 dark:text-gray-300"
                                title="Open File"
                                on:click=move |ev| { ev.stop_propagation(); core.on_get_doc_diff.run(path_for_open.clone()); }
                            >
                                <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/><polyline points="15 3 21 3 21 9"/><line x1="10" y1="14" x2="21" y2="3"/></svg>
                            </button>
                            <button
                                class="p-0.5 hover:bg-[#d0d0d0] dark:hover:bg-[#454545] rounded text-gray-600 dark:text-gray-300"
                                title="Discard Changes"
                                on:click=move |ev| { ev.stop_propagation(); core.on_discard_file.run(path_for_discard.clone()); }
                            >
                                <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 3v5h5"/><path d="M3.05 13A9 9 0 1 0 6 5.3L3 8"/></svg>
                            </button>
                            <button
                                class="p-0.5 hover:bg-[#d0d0d0] dark:hover:bg-[#454545] rounded text-gray-600 dark:text-gray-300"
                                title="Stage Changes"
                                on:click=move |ev| { ev.stop_propagation(); core.on_stage_file.run(path_for_stage.clone()); }
                            >
                                <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>
                            </button>
                        }.into_any()
                    }}
                </div>

                // 状态标记 (M/A/D)
                <span class=format!("{} text-[11px] font-bold w-3 text-center", color_cls)>
                    {icon_char}
                </span>
            </div>
        </div>
    }
}
