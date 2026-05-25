// apps\web\src\components
//! plan_ref:
//!   - 18_release#runtime-observability
//!   - 11_ui_design/01_web#web-layout-persistence
//!
//! # BottomBar 组件 (BottomBar Component)
//!
//! 底部状态栏，显示分支切换器、连接状态和编辑器统计信息 (字数、行数、字符数)。

mod stats;
mod status;
mod time_travel;

use crate::components::branch_switcher::BranchSwitcher;
use crate::editor::EditorStats;
use crate::hooks::use_core::CoreState;
use crate::i18n::Locale;
use leptos::prelude::*;

use self::stats::BottomBarStats;
use self::status::BottomBarStatus;
use self::time_travel::BottomBarTimeTravel;

#[component]
pub fn BottomBar(core: CoreState) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let stats = core.stats;
    let max_ver = core.doc_version;
    let curr_ver = core.playback_version;
    let set_ver = core.set_playback_version;
    let current_doc = core.current_doc;
    let load_state = core.load_state;
    let load_progress = core.load_progress;
    let load_eta_ms = core.load_eta_ms;
    let displayed_stats = Signal::derive(move || {
        if current_doc.get().is_some() {
            stats.get()
        } else {
            EditorStats::default()
        }
    });
    let displayed_max_ver = Signal::derive(move || {
        if current_doc.get().is_some() {
            max_ver.get()
        } else {
            0
        }
    });
    let displayed_curr_ver = Signal::derive(move || {
        if current_doc.get().is_some() {
            curr_ver.get()
        } else {
            0
        }
    });

    view! {
        <footer class="h-8 bg-sidebar border-t border-default flex items-center justify-between px-4 select-none relative">
            <div class="flex items-center gap-3">
                <BranchSwitcher />
                <div class="w-px h-4 bg-active"></div>
                <BottomBarStatus core=core.clone() locale />
            </div>

            <BottomBarTimeTravel
                locale
                displayed_curr_ver
                displayed_max_ver
                set_ver
            />

            <BottomBarStats
                locale
                displayed_stats
                load_state
                load_progress
                load_eta_ms
            />
        </footer>
    }
}
