//! plan_ref:
//!   - 08_auth#auth-http-endpoints
//!   - 08_auth#auth-rate-limiting
//!

use deve_core::protocol::auth::{AuthErrorCode, LoginRequest, LoginResponse};
use gloo_net::http::Request;
use web_sys::RequestCredentials;

use crate::api::{AuthProbe, api_url, probe_auth_status};

#[derive(Debug)]
pub(super) enum LoginAttemptError {
    Rejected(AuthErrorCode),
    InvalidResponse,
    Transport(LoginTransportError),
}

#[derive(Debug)]
pub(super) enum LoginTransportError {
    RequestBuild(String),
    Network(String),
}

pub(super) async fn attempt_login(
    username: String,
    password: String,
) -> Result<(), LoginAttemptError> {
    let request = LoginRequest { username, password };
    let api = api_url("/api/auth/login");
    let mut builder = Request::post(&api.url);
    if api.include_credentials {
        builder = builder.credentials(RequestCredentials::Include);
    }
    let response = builder
        .header("Content-Type", "application/json")
        .json(&request)
        .map_err(|e| {
            LoginAttemptError::Transport(LoginTransportError::RequestBuild(e.to_string()))
        })?
        .send()
        .await
        .map_err(|e| LoginAttemptError::Transport(LoginTransportError::Network(e.to_string())))?;
    let status = response.status();
    let payload = response.json::<LoginResponse>().await.ok();
    if let Some(result) = payload {
        if result.success {
            return match probe_auth_status().await {
                AuthProbe::Valid => Ok(()),
                AuthProbe::Invalid | AuthProbe::Unknown => Err(LoginAttemptError::InvalidResponse),
            };
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

pub async fn logout() -> Result<(), String> {
    let api = api_url("/api/auth/logout");
    let mut request = Request::post(&api.url);
    if api.include_credentials {
        request = request.credentials(RequestCredentials::Include);
    }
    let response = request.send().await.map_err(|e| e.to_string())?;
    if response.ok() {
        Ok(())
    } else {
        Err(format!("HTTP {}", response.status()))
    }
}
