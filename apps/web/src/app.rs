// apps/web/src/app.rs
//! plan_ref:
//!   - 09_auth#session-probe-policy
//!   - 09_auth#unauthorized-disconnected-ui
//!
//! # App Component (App 组件)
//!
//! 本模块包含根 `App` 组件。
//! 主布局逻辑已移动至 `components/main_layout.rs`。
//!
//! ## 结构说明
//!
//! - `App`: 根组件，提供语言环境上下文和认证状态管理。

use self::app_auth_monitor::{
    current_page_active, mount_visibility_listener, should_run_session_probe,
};
use crate::api::{AuthProbe, probe_auth_status};
use crate::components::login::{AuthState, AuthUnavailablePage, LoginPage};
use crate::components::main_layout::MainLayout;
use crate::i18n::Locale;
use gloo_timers::future::TimeoutFuture;
use leptos::prelude::*;
use leptos::task::spawn_local;

#[path = "app_auth_monitor.rs"]
mod app_auth_monitor;

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
    let (auth_state, set_auth_state) = signal(AuthState::Checking);
    let page_active = RwSignal::new(current_page_active());

    window_event_listener(leptos::ev::focus, move |_| {
        page_active.set(current_page_active());
    });
    window_event_listener(leptos::ev::blur, move |_| {
        page_active.set(current_page_active());
    });
    mount_visibility_listener(page_active);

    let last_page_active = StoredValue::new_local(page_active.get_untracked());
    Effect::new(move |_| {
        let active = page_active.get();
        let was_active = last_page_active.get_value();
        last_page_active.set_value(active);
        if was_active || !should_run_session_probe(&auth_state.get(), active) {
            return;
        }
        spawn_local(async move {
            match probe_auth_status().await {
                AuthProbe::Invalid => set_auth_state.set(AuthState::Unauthenticated),
                AuthProbe::Valid | AuthProbe::Unknown => {}
            }
        });
    });

    // 启动时检查认证状态
    spawn_local(async move {
        loop {
            match probe_auth_status().await {
                AuthProbe::Valid => {
                    set_auth_state.set(AuthState::Authenticated);
                    break;
                }
                AuthProbe::Invalid => {
                    set_auth_state.set(AuthState::Unauthenticated);
                    break;
                }
                AuthProbe::Unknown => {
                    set_auth_state.set(AuthState::Unavailable);
                    TimeoutFuture::new(AUTH_MONITOR_MS).await;
                }
            }
        }
    });

    // 已登录状态下定期探测认证是否仍然有效。
    // 网络错误保持当前 UI，不把断线误判成 session 失效。
    spawn_local(async move {
        loop {
            TimeoutFuture::new(AUTH_MONITOR_MS).await;
            if !should_run_session_probe(&auth_state.get_untracked(), page_active.get_untracked()) {
                continue;
            }
            match probe_auth_status().await {
                AuthProbe::Valid => {}
                AuthProbe::Invalid => set_auth_state.set(AuthState::Unauthenticated),
                AuthProbe::Unknown => {}
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
            AuthState::Checking | AuthState::Unavailable => view! {
                <AuthUnavailablePage />
            }.into_any(),
            _ => view! {
                <LoginPage auth_state=auth_state set_auth_state=set_auth_state/>
            }.into_any(),
        }}
    }
}
