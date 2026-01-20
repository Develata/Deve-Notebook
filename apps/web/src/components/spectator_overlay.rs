// apps\web\src\components
//! # Spectator Overlay 组件 (旁观者模式覆盖层)
//!
//! 当用户查看 Shadow Repo 时显示只读指示器。
//! 符合 `03_ui_architecture.md` Section 1.x 规范。

use crate::hooks::use_core::use_core;
use crate::i18n::Locale;
use leptos::prelude::*;

/// 旁观者模式覆盖层
/// - 订阅 `core.is_spectator` 信号
/// - 渲染灰色水印覆盖层 + "READ ONLY" 指示器
#[component]
pub fn SpectatorOverlay() -> impl IntoView {
    let core = use_core();
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");

    view! {
        <Show when=move || core.is_spectator.get()>
            // 全屏半透明覆盖层
            <div class="fixed inset-0 z-[40] pointer-events-none select-none">
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
                        "READ ONLY"
                    </div>
                </div>
            </div>

            // 底部状态栏指示器 (需要 pointer-events 以允许交互)
            // 固定高度 h-8 (32px) 以避免遮挡不确定区域
            <div class="fixed bottom-0 left-0 right-0 z-[50] bg-amber-500 text-white h-8 flex items-center justify-center pointer-events-auto text-sm font-semibold">
                <span class="flex items-center gap-2">
                    <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" />
                    </svg>
                    {move || if locale.get() == Locale::Zh {
                        "旁观者模式 - 只读"
                    } else {
                        "Spectator Mode - Read Only"
                    }}
                </span>
            </div>
        </Show>
    }
}
