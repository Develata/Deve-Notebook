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
use crate::hooks::use_core::EditorContext;
use crate::i18n::Locale;
use crate::runtime::{document_client::DocumentClient, rendering_client::RenderingClient};
use leptos::prelude::*;

use self::stats::BottomBarStats;
use self::status::BottomBarStatus;
use self::time_travel::BottomBarTimeTravel;

#[component]
pub fn BottomBar() -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let rendering = expect_context::<RenderingClient>();
    let editor = expect_context::<EditorContext>();
    let document = expect_context::<DocumentClient>();
    let stats = rendering.stats;
    let max_ver = editor.doc_version;
    let curr_ver = editor.playback_version;
    let set_ver = editor.set_playback_version;
    let current_doc = document.current_doc;
    let load_state = rendering.load_state;
    let load_progress = rendering.load_progress;
    let load_eta_ms = rendering.load_eta_ms;
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
                <BottomBarStatus locale />
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
