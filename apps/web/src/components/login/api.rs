use deve_core::protocol::auth::{AuthErrorCode, LoginRequest, LoginResponse};
use gloo_net::http::Request;

#[derive(Debug)]
pub(super) enum LoginAttemptError {
    Rejected(AuthErrorCode),
    InvalidResponse,
    Transport(String),
}

pub(super) async fn attempt_login(
    username: String,
    password: String,
) -> Result<(), LoginAttemptError> {
    let request = LoginRequest { username, password };
    let response = Request::post("/api/auth/login")
        .header("Content-Type", "application/json")
        .json(&request)
        .map_err(|e| LoginAttemptError::Transport(format!("请求构建失败: {}", e)))?
        .send()
        .await
        .map_err(|e| LoginAttemptError::Transport(format!("网络错误: {}", e)))?;
    let status = response.status();
    let payload = response.json::<LoginResponse>().await.ok();
    if let Some(result) = payload {
        if result.success {
            return Ok(());
        }
        if let Some(code) = result.code {
            return Err(LoginAttemptError::Rejected(code));
        }
    }
    match status {
        401 => Err(LoginAttemptError::Rejected(AuthErrorCode::InvalidPassword)),
        429 => Err(LoginAttemptError::Rejected(AuthErrorCode::RateLimited)),
        500 => Err(LoginAttemptError::Rejected(AuthErrorCode::InternalError)),
        _ => Err(LoginAttemptError::InvalidResponse),
    }
}

#[allow(dead_code)]
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
