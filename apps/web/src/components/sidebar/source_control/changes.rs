// apps\web\src\components\sidebar\source_control
//! # Changes 组件 (变更列表组件)
//!
//! 组合 `StagedSection` 和 `UnstagedSection` 子组件，
//! 显示完整的变更列表视图。

use crate::hooks::use_core::CoreState;
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
    let core = expect_context::<CoreState>();

    // 初次加载时获取变更数据
    Effect::new(move |_| {
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
