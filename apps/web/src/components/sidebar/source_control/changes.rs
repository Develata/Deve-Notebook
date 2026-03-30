// apps\web\src\components\sidebar\source_control
//! # Changes 组件 (变更列表组件)
//!
//! 组合 `StagedSection` 和 `UnstagedSection` 子组件，
//! 显示完整的变更列表视图。

use crate::hooks::use_core::SourceControlContext;
use leptos::prelude::*;

use super::staged_section::StagedSection;
use super::unstaged_section::UnstagedSection;

/// 变更列表主组件
///
/// 职责:
/// - 触发变更数据获取
/// - 分发数据到子组件
#[component]
pub fn Changes() -> impl IntoView {
    let core = expect_context::<SourceControlContext>();
    let write_block = core.write_block;

    Effect::new(move |_| {
        if core.current_repo_id.get().is_none()
            || core.active_branch.get().is_some()
            || core.pending_branch_switch.get().is_some()
            || core.pending_repo_switch.get().is_some()
            || write_block.get().is_some()
        {
            return;
        }
        core.on_get_changes.run(());
    });

    view! {
        <div>
            {move || {
                let staged = core.staged_changes.get();
                let unstaged = core.unstaged_changes.get();

                view! {
                    <div>
                        <StagedSection staged=staged />
                        <UnstagedSection unstaged=unstaged />
                    </div>
                }
            }}
        </div>
    }
}
