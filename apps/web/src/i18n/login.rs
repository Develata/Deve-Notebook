//! 登录页与认证错误文案。
//! plan_ref:
//!   - 11_i18n#i18n-keys-reference
//!   - 11_i18n#i18n-error-code-catalog

use super::Locale;
use deve_core::protocol::auth::AuthErrorCode;

pub fn app_name(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Deve Notebook",
        Locale::Zh => "Deve 笔记",
    }
}

pub fn subtitle(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Sign in to continue",
        Locale::Zh => "登录以继续",
    }
}

pub fn username(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Username",
        Locale::Zh => "用户名",
    }
}

pub fn password(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Password",
        Locale::Zh => "密码",
    }
}

pub fn username_placeholder(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Enter username",
        Locale::Zh => "输入用户名",
    }
}

pub fn password_placeholder(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Enter password",
        Locale::Zh => "输入密码",
    }
}

pub fn button(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Sign In",
        Locale::Zh => "登录",
    }
}

pub fn loading(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Signing in...",
        Locale::Zh => "登录中...",
    }
}

pub fn empty_fields(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Enter username and password",
        Locale::Zh => "请输入用户名和密码",
    }
}

pub fn invalid_response(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Login response missing success/code field",
        Locale::Zh => "登录响应缺少 success/code 字段",
    }
}

pub fn transport_error(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Login error",
        Locale::Zh => "登录错误",
    }
}

pub fn request_build_error(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Request build failed",
        Locale::Zh => "请求构建失败",
    }
}

pub fn network_error(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Network error",
        Locale::Zh => "网络错误",
    }
}

pub fn auth_unavailable_hint(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Unable to reach the auth service. We'll retry automatically.",
        Locale::Zh => "当前无法连接认证服务，系统会自动重试。",
    }
}

pub fn auth_error(locale: Locale, code: AuthErrorCode) -> &'static str {
    match (locale, code) {
        (Locale::En, AuthErrorCode::InvalidPassword) => "Invalid username or password",
        (Locale::Zh, AuthErrorCode::InvalidPassword) => "用户名或密码错误",
        (Locale::En, AuthErrorCode::TokenExpired) => "Session expired",
        (Locale::Zh, AuthErrorCode::TokenExpired) => "登录状态已过期",
        (Locale::En, AuthErrorCode::TokenMissing) => "Authentication required",
        (Locale::Zh, AuthErrorCode::TokenMissing) => "需要登录",
        (Locale::En, AuthErrorCode::RateLimited) => "Too many attempts, try again later",
        (Locale::Zh, AuthErrorCode::RateLimited) => "尝试次数过多，请稍后再试",
        (Locale::En, AuthErrorCode::CsrfMismatch) => "Request rejected",
        (Locale::Zh, AuthErrorCode::CsrfMismatch) => "请求被拒绝",
        (Locale::En, AuthErrorCode::InternalError) => "Internal server error",
        (Locale::Zh, AuthErrorCode::InternalError) => "服务器内部错误",
    }
}
