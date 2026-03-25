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
use leptos::prelude::*;
use leptos::task::spawn_local;

fn dev_bypass_login() -> bool {
    cfg!(debug_assertions)
}

/// 根应用程序组件
///
/// 初始化全局状态 (Locale) 并处理认证状态切换：
/// - 未认证时显示登录页
/// - 已认证时显示主布局
#[component]
pub fn App() -> impl IntoView {
    // 全局语言环境状态
    let locale = RwSignal::new(Locale::default());
    provide_context(locale);
    let debug_bypass = dev_bypass_login();

    // 认证状态
    let (auth_state, set_auth_state) = signal(if debug_bypass {
        leptos::logging::log!("Debug auth bypass enabled: always rendering MainLayout");
        AuthState::Authenticated
    } else {
        AuthState::Unauthenticated
    });

    if !debug_bypass {
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
    }

    view! {
        {move || if debug_bypass {
            view! {
                <MainLayout
                    on_session_expired=Callback::new(move |_| {
                        leptos::logging::log!("Debug auth bypass ignored session expiration");
                    })
                />
            }.into_any()
        } else {
            match auth_state.get() {
                AuthState::Authenticated => view! {
                    <MainLayout
                        on_session_expired=Callback::new(move |_| set_auth_state.set(AuthState::Unauthenticated))
                    />
                }.into_any(),
                _ => view! {
                    <LoginPage auth_state=auth_state set_auth_state=set_auth_state/>
                }.into_any(),
            }
        }}
    }
}
