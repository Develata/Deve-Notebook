// apps/web/src/components/login.rs
//! # 登录页面组件 (Login Page)
//!
//! 处理用户认证，包括登录表单、401 状态显示。

use crate::i18n::{Locale, t};
use gloo_net::http::Request;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

/// 登录请求体
#[derive(Serialize)]
struct LoginRequest {
    username: String,
    password: String,
}

/// 登录响应
#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
enum LoginStatus {
    Success,
    InvalidCredentials,
}

#[derive(Deserialize, Debug)]
struct LoginResponse {
    status: LoginStatus,
}

/// 认证状态
#[derive(Clone, Copy, PartialEq)]
pub enum AuthState {
    /// 未认证
    Unauthenticated,
    /// 认证中
    Authenticating,
    /// 已认证
    Authenticated,
    /// 认证失败（显示错误信息）
    Failed(String),
}

/// 登录页面组件
#[component]
pub fn LoginPage(auth_state: ReadSignal<AuthState>, set_auth_state: WriteSignal<AuthState>) -> impl IntoView {
    let (username, set_username) = signal(String::new());
    let (password, set_password) = signal(String::new());
    let (is_loading, set_is_loading) = signal(false);
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));

    // 错误信息派生信号
    let error_message = move || match auth_state.get() {
        AuthState::Failed(msg) => Some(msg),
        _ => None,
    };

    // 执行登录
    let do_login = move |_| {
        let user = username.get();
        let pass = password.get();
        
        if user.is_empty() || pass.is_empty() {
            set_auth_state.set(AuthState::Failed("请输入用户名和密码".to_string()));
            return;
        }

        set_is_loading.set(true);
        set_auth_state.set(AuthState::Authenticating);

        spawn_local(async move {
            let result = attempt_login(user, pass).await;
            set_is_loading.set(false);
            
            match result {
                Ok(true) => {
                    set_auth_state.set(AuthState::Authenticated);
                }
                Ok(false) => {
                    set_auth_state.set(AuthState::Failed(
                        t::common::login_failed(locale.get()).to_string()
                    ));
                }
                Err(e) => {
                    set_auth_state.set(AuthState::Failed(format!("{}: {}", 
                        t::common::login_error(locale.get()), 
                        e
                    )));
                }
            }
        });
    };

    view! {
        <div class="fixed inset-0 bg-bg flex items-center justify-center z-50">
            <div class="w-full max-w-sm p-8 bg-bg-panel rounded-lg shadow-lg border border-border">
                <h1 class="text-2xl font-bold text-center text-primary mb-2">
                    {move || t::common::app_name(locale.get())}
                </h1>
                <p class="text-sm text-muted text-center mb-6">
                    {move || t::common::login_subtitle(locale.get())}
                </p>

                // 错误信息显示
                {move || error_message().map(|msg| view! {
                    <div class="mb-4 p-3 bg-red-900/20 border border-red-500/30 rounded text-red-400 text-sm">
                        {msg}
                    </div>
                })}

                <div class="space-y-4">
                    // 用户名输入
                    <div>
                        <label class="block text-xs font-medium text-muted mb-1">
                            {move || t::common::username(locale.get())}
                        </label>
                        <input
                            type="text"
                            class="w-full px-3 py-2 bg-bg border border-border rounded text-primary text-sm focus:outline-none focus:border-accent"
                            prop:value=move || username.get()
                            on:input=move |e| set_username.set(event_target_value(&e))
                            on:keypress=move |e: leptos::ev::KeyboardEvent| {
                                if e.key() == "Enter" {
                                    do_login(());
                                }
                            }
                            placeholder=move || t::common::username_placeholder(locale.get())
                        />
                    </div>

                    // 密码输入
                    <div>
                        <label class="block text-xs font-medium text-muted mb-1">
                            {move || t::common::password(locale.get())}
                        </label>
                        <input
                            type="password"
                            class="w-full px-3 py-2 bg-bg border border-border rounded text-primary text-sm focus:outline-none focus:border-accent"
                            prop:value=move || password.get()
                            on:input=move |e| set_password.set(event_target_value(&e))
                            on:keypress=move |e: leptos::ev::KeyboardEvent| {
                                if e.key() == "Enter" {
                                    do_login(());
                                }
                            }
                            placeholder=move || t::common::password_placeholder(locale.get())
                        />
                    </div>

                    // 登录按钮
                    <button
                        class=move || format!(
                            "w-full py-2 px-4 rounded font-medium text-sm transition-colors {}",
                            if is_loading.get() {
                                "bg-accent/50 cursor-not-allowed text-white"
                            } else {
                                "bg-accent hover:bg-accent/80 text-white"
                            }
                        )
                        on:click=do_login
                        disabled=move || is_loading.get() || username.get().is_empty() || password.get().is_empty()
                    >
                        {move || if is_loading.get() {
                            t::common::logging_in(locale.get()).to_string()
                        } else {
                            t::common::login_button(locale.get()).to_string()
                        }}
                    </button>
                </div>
            </div>
        </div>
    }
}

/// 尝试登录
async fn attempt_login(username: String, password: String) -> Result<bool, String> {
    let request = LoginRequest { username, password };
    
    let response = Request::post("/api/auth/login")
        .header("Content-Type", "application/json")
        .json(&request)
        .map_err(|e| format!("请求构建失败: {}", e))?
        .send()
        .await
        .map_err(|e| format!("网络错误: {}", e))?;

    if response.status() == 401 {
        return Ok(false);
    }

    if !response.ok() {
        return Err(format!("HTTP {}", response.status()));
    }

    let result: LoginResponse = response
        .json()
        .await
        .map_err(|e| format!("解析响应失败: {}", e))?;

    Ok(matches!(result.status, LoginStatus::Success))
}

/// 检查当前认证状态
pub async fn check_auth_status() -> Result<bool, String> {
    let response = Request::get("/api/auth/me")
        .send()
        .await
        .map_err(|e| format!("网络错误: {}", e))?;

    Ok(response.ok())
}

/// 登出
pub async fn logout() -> Result<(), String> {
    let response = Request::post("/api/auth/logout")
        .send()
        .await
        .map_err(|e| format!("网络错误: {}", e))?;

    if response.ok() {
        Ok(())
    } else {
        Err(format!("HTTP {}", response.status()))
    }
}
