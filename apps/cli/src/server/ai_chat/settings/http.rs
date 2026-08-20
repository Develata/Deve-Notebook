//! plan_ref:
//!   - 13_i18n#i18n-error-code-catalog
//!   - 15_settings#native-ai-provider-settings
//!   - 08_auth#auth-http-endpoints

use super::{ProviderSettingsProjection, ReplaceError, ReplaceProviderSettings};
use crate::server::AppState;
use axum::{
    Json,
    extract::{State, rejection::JsonRejection},
    http::StatusCode,
};
use serde::Serialize;
use std::sync::Arc;

#[derive(Clone, Copy, Serialize)]
pub(crate) enum AiSettingsErrorCode {
    #[serde(rename = "AI_SETTINGS_ENVIRONMENT_MANAGED")]
    EnvironmentManaged,
    #[serde(rename = "AI_SETTINGS_REVISION_CONFLICT")]
    RevisionConflict,
    #[serde(rename = "AI_SETTINGS_INVALID")]
    Invalid,
    #[serde(rename = "AI_SETTINGS_UNAVAILABLE")]
    Unavailable,
}

#[derive(Serialize)]
pub(crate) struct ErrorBody {
    code: AiSettingsErrorCode,
}

pub(crate) async fn get(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ProviderSettingsProjection>, (StatusCode, Json<ErrorBody>)> {
    state
        .ai_provider_settings()
        .projection()
        .map(Json)
        .map_err(|_| internal_error())
}

pub(crate) async fn replace(
    State(state): State<Arc<AppState>>,
    request: Result<Json<ReplaceProviderSettings>, JsonRejection>,
) -> Result<Json<ProviderSettingsProjection>, (StatusCode, Json<ErrorBody>)> {
    let Json(request) = request.map_err(|_| invalid_request_error())?;
    let runtime = state.ai_provider_settings();
    tokio::task::spawn_blocking(move || runtime.replace(request))
        .await
        .map_err(|_| internal_error())?
        .map(Json)
        .map_err(map_replace_error)
}

fn invalid_request_error() -> (StatusCode, Json<ErrorBody>) {
    error(StatusCode::BAD_REQUEST, AiSettingsErrorCode::Invalid)
}

fn map_replace_error(failure: ReplaceError) -> (StatusCode, Json<ErrorBody>) {
    match failure {
        ReplaceError::EnvironmentManaged => error(
            StatusCode::CONFLICT,
            AiSettingsErrorCode::EnvironmentManaged,
        ),
        ReplaceError::RevisionConflict => {
            error(StatusCode::CONFLICT, AiSettingsErrorCode::RevisionConflict)
        }
        ReplaceError::Invalid => error(StatusCode::BAD_REQUEST, AiSettingsErrorCode::Invalid),
        ReplaceError::Persistence | ReplaceError::Internal => internal_error(),
    }
}

fn internal_error() -> (StatusCode, Json<ErrorBody>) {
    error(
        StatusCode::INTERNAL_SERVER_ERROR,
        AiSettingsErrorCode::Unavailable,
    )
}

fn error(status: StatusCode, code: AiSettingsErrorCode) -> (StatusCode, Json<ErrorBody>) {
    (status, Json(ErrorBody { code }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ai_settings_error_status_code_matrix_is_closed() {
        let cases = [
            (
                ReplaceError::EnvironmentManaged,
                StatusCode::CONFLICT,
                "AI_SETTINGS_ENVIRONMENT_MANAGED",
            ),
            (
                ReplaceError::RevisionConflict,
                StatusCode::CONFLICT,
                "AI_SETTINGS_REVISION_CONFLICT",
            ),
            (
                ReplaceError::Invalid,
                StatusCode::BAD_REQUEST,
                "AI_SETTINGS_INVALID",
            ),
            (
                ReplaceError::Persistence,
                StatusCode::INTERNAL_SERVER_ERROR,
                "AI_SETTINGS_UNAVAILABLE",
            ),
            (
                ReplaceError::Internal,
                StatusCode::INTERNAL_SERVER_ERROR,
                "AI_SETTINGS_UNAVAILABLE",
            ),
        ];

        for (failure, expected_status, expected_code) in cases {
            let (status, Json(body)) = map_replace_error(failure);
            assert_eq!(status, expected_status);
            assert_eq!(
                serde_json::to_value(body).expect("serialize closed error body"),
                serde_json::json!({ "code": expected_code })
            );
        }
    }
}
