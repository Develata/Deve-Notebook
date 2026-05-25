// apps\web\src\components
//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 09_web_thin_client_ledger#web-edit-intent
//!
//! # Spectator Overlay 组件 (旁观者模式覆盖层)
//!
//! 当用户查看 Shadow Repo 时显示只读指示器。
//! 符合 `03_ui_architecture.md` Section 1.x 规范。

use crate::components::icons::Lock;
use crate::hooks::use_core::EditorContext;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

/// 旁观者模式覆盖层
/// - 订阅 `core.is_spectator` 信号
/// - 渲染灰色水印覆盖层 + "READ ONLY" 指示器
#[component]
pub fn SpectatorOverlay() -> impl IntoView {
    let core = expect_context::<EditorContext>();
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");

    view! {
        <Show when=move || core.is_spectator.get()>
            // 全屏半透明覆盖层
            <div class="fixed inset-0 z-[var(--z-chrome)] pointer-events-none select-none">
                // 斜纹水印背景
                <div
                    class="absolute inset-0 bg-gray-900/5"
                    style="background-image: repeating-linear-gradient(
                        45deg,
                        transparent,
                        transparent 10px,
                        rgba(0, 0, 0, 0.03) 10px,
                        rgba(0, 0, 0, 0.03) 20px
                    );"
                />

                // 中央水印文字
                <div class="absolute inset-0 flex items-center justify-center">
                    <div class="text-gray-400/20 text-9xl font-black uppercase tracking-widest transform -rotate-12 select-none">
                        {move || t::common::read_only_watermark(locale.get())}
                    </div>
                </div>
            </div>

            // 底部状态栏指示器 (需要 pointer-events 以允许交互)
            // 固定高度 h-8 (32px) 以避免遮挡不确定区域
            <div class="fixed bottom-0 left-0 right-0 z-[var(--z-panels)] bg-amber-500 text-white h-8 flex items-center justify-center pointer-events-auto text-sm font-semibold">
                <span class="flex items-center gap-2">
                    <Lock class="w-4 h-4"/>
                    {move || t::common::spectator_status(locale.get())}
                </span>
            </div>
        </Show>
    }
}
