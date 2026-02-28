// apps\web\src\components\branch_switcher
//! # Branch Switcher Component (分支切换器)
//!
//! **架构作用**:
//! VS Code 风格的分支切换器，显示在状态栏左侧，
//! 允许用户在本地仓库 (Local/Master) 和远端影子库 (Shadow/Peer-XXX) 之间切换。
//!
//! **核心功能**:
//! - 显示当前活动分支
//! - 下拉列表显示所有可用分支
//! - 远端分支以只读模式 (Spectator Mode) 标识
//!
//! **类型**: Core MAY (扩展可选)

use crate::components::main_layout::SearchControl;
use crate::hooks::use_core::CoreState;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

#[component]
pub fn BranchSwitcher(#[prop(optional)] compact: bool) -> impl IntoView {
    let core = expect_context::<CoreState>();
    let search_control = expect_context::<SearchControl>();
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");

    // 挂载时请求 Shadow 列表
    Effect::new(move |_| {
        core.on_list_shadows.run(());
    });

    // 获取当前分支名称
    let current_branch = move || match core.active_branch.get() {
        None => {
            if compact {
                t::sidebar::local_branch(locale.get()).to_string()
            } else {
                t::sidebar::local_master_branch(locale.get()).to_string()
            }
        }
        Some(peer) => {
            let s = peer.to_string();
            if compact { compact_branch_name(&s) } else { s }
        }
    };

    // 判断是否为 Spectator (只读) 模式
    let is_spectator = move || core.active_branch.get().is_some();

    // 点击打开 Unified Search 并切换到 Branch Mode (@)
    let onclick = move |_| {
        search_control.set_mode.set("@".to_string());
        search_control.set_show.set(true);
    };

    view! {
        <button
            class=move || {
                if compact {
                    "flex items-center gap-1 px-1.5 py-0.5 rounded border border-gray-200 bg-gray-50 hover:bg-gray-100 transition-colors duration-200 ease-out text-[10px] font-medium text-gray-700 max-w-[96px]"
                } else {
                    "flex items-center gap-1.5 px-2 py-0.5 rounded hover:bg-gray-200 transition-colors duration-200 ease-out text-xs font-medium text-gray-700"
                }
            }
            on:click=onclick
            title="Switch Branch (Ctrl+Shift+L)"
        >
            // Git 分支图标
            <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <line x1="6" y1="3" x2="6" y2="15"/>
                <circle cx="18" cy="6" r="3"/>
                <circle cx="6" cy="18" r="3"/>
                <path d="M18 9a9 9 0 0 1-9 9"/>
            </svg>
            <span class=move || if compact { "truncate max-w-[60px]" } else { "" }>{current_branch}</span>
            {move || if is_spectator() && !compact {
                view! { <span class="text-[10px] bg-amber-100 text-amber-700 px-1 rounded font-normal">"READ"</span> }.into_any()
            } else {
                view! {}.into_any()
            }}
        </button>
    }
}

fn compact_branch_name(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    if chars.len() <= 13 {
        return name.to_string();
    }
    let head: String = chars.iter().take(7).collect();
    let tail: String = chars
        .iter()
        .rev()
        .take(4)
        .cloned()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("{}...{}", head, tail)
}
