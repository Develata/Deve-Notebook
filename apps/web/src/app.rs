// apps/web/src/app.rs
//! # App Component (App 组件)
//!
//! 本模块包含根 `App` 组件。
//! 主布局逻辑已移动至 `components/main_layout.rs`。
//!
//! ## 结构说明
//!
//! - `App`: 根组件，提供语言环境上下文和认证状态管理。

use crate::components::login::{AuthState, LoginPage, check_auth_status};
use crate::components::main_layout::MainLayout;
use crate::i18n::Locale;
use gloo_timers::future::TimeoutFuture;
use leptos::prelude::*;
use leptos::task::spawn_local;

/// 根应用程序组件
///
/// 初始化全局状态 (Locale) 并处理认证状态切换：
/// - 未认证时显示登录页
/// - 已认证时显示主布局
#[component]
pub fn App() -> impl IntoView {
    const AUTH_MONITOR_MS: u32 = 5_000;

    // 全局语言环境状态
    let locale = RwSignal::new(Locale::default());
    provide_context(locale);

    // 认证状态
    let (auth_state, set_auth_state) = signal(AuthState::Unauthenticated);

    // 启动时检查认证状态
    spawn_local(async move {
        match check_auth_status().await {
            Ok(true) => set_auth_state.set(AuthState::Authenticated),
            Ok(false) => set_auth_state.set(AuthState::Unauthenticated),
            Err(e) => {
                leptos::logging::warn!("Auth check failed: {}", e);
                set_auth_state.set(AuthState::Unauthenticated);
            }
        }
    });

    // 已登录状态下定期探测认证是否仍然有效。
    // 网络错误保持当前 UI，不把断线误判成 session 失效。
    spawn_local(async move {
        loop {
            TimeoutFuture::new(AUTH_MONITOR_MS).await;
            if auth_state.get_untracked() != AuthState::Authenticated {
                continue;
            }
            match check_auth_status().await {
                Ok(true) => {}
                Ok(false) => set_auth_state.set(AuthState::Unauthenticated),
                Err(_) => {}
            }
        }
    });

    view! {
        {move || match auth_state.get() {
            AuthState::Authenticated => view! {
                <MainLayout
                    on_session_expired=Callback::new(move |_| set_auth_state.set(AuthState::Unauthenticated))
                />
            }.into_any(),
            _ => view! {
                <LoginPage auth_state=auth_state set_auth_state=set_auth_state/>
            }.into_any(),
        }}
    }
}
