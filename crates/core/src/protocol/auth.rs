//! HTTP 认证协议共享类型。
//!
//! Invariants:
//! - 登录相关 HTTP 端点只返回结构化 JSON，不返回自然语言错误。
//! - 错误码由前后端共享，避免契约漂移。

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthErrorCode {
    #[serde(rename = "AUTH_INVALID_PASSWORD")]
    InvalidPassword,
    #[serde(rename = "AUTH_TOKEN_EXPIRED")]
    TokenExpired,
    #[serde(rename = "AUTH_TOKEN_MISSING")]
    TokenMissing,
    #[serde(rename = "AUTH_RATE_LIMITED")]
    RateLimited,
    #[serde(rename = "AUTH_CSRF_MISMATCH")]
    CsrfMismatch,
    #[serde(rename = "AUTH_INTERNAL_ERROR")]
    InternalError,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthErrorResponse {
    pub code: AuthErrorCode,
}

impl AuthErrorResponse {
    pub const fn new(code: AuthErrorCode) -> Self {
        Self { code }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoginResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<AuthErrorCode>,
}

impl LoginResponse {
    pub const fn success() -> Self {
        Self {
            success: true,
            code: None,
        }
    }

    pub const fn failure(code: AuthErrorCode) -> Self {
        Self {
            success: false,
            code: Some(code),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeResponse {
    pub username: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthStatusResponse {
    pub authenticated: bool,
}

impl AuthStatusResponse {
    pub const fn authenticated() -> Self {
        Self {
            authenticated: true,
        }
    }

    pub const fn unauthenticated() -> Self {
        Self {
            authenticated: false,
        }
    }
}
