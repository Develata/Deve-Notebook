//! plan_ref:
//!   - 08_ui_design_01_web#web-layout-persistence
//!
//! # Lucide-style SVG Icon Components
//!
//! 轻量级图标组件（基于 Lucide Design），零外部依赖。
//! 每个图标遵循 Lucide 24×24 viewBox 规范，使用 `currentColor` 继承父级颜色。

/// SVG 公共属性宏 (减少重复)
///
/// Invariant: 所有图标共享 24×24 viewBox、stroke="currentColor"、stroke-width="2"。
/// 子模块通过父模块作用域自动继承此宏。
macro_rules! icon {
    ($name:ident, $body:expr) => {
        #[component]
        pub fn $name(
            #[prop(default = "w-4 h-4".to_string(), into)] class: String,
        ) -> impl IntoView {
            let cls = class;
            view! {
                <svg class=cls xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"
                    fill="none" stroke="currentColor" stroke-width="2"
                    stroke-linecap="round" stroke-linejoin="round">
                    {$body}
                </svg>
            }
        }
    };
}

mod common;
mod git;
mod navigation;
mod ui;

pub use common::*;
pub use git::*;
pub use navigation::*;
pub use ui::*;
