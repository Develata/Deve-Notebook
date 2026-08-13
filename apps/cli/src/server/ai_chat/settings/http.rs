//! plan_ref:
//!   - 15_settings#native-ai-provider-settings
//!   - 08_auth#auth-http-endpoints

use super::{ProviderSettingsProjection, ReplaceError, ReplaceProviderSettings};
use crate::server::AppState;
use axum::{Json, extract::State, http::StatusCode};
use serde::Serialize;
use std::sync::Arc;

#[derive(Serialize)]
pub(crate) struct ErrorBody {
    error: &'static str,
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
    Json(request): Json<ReplaceProviderSettings>,
) -> Result<Json<ProviderSettingsProjection>, (StatusCode, Json<ErrorBody>)> {
    let runtime = state.ai_provider_settings();
    tokio::task::spawn_blocking(move || runtime.replace(request))
        .await
        .map_err(|_| internal_error())?
        .map(Json)
        .map_err(map_replace_error)
}

fn map_replace_error(failure: ReplaceError) -> (StatusCode, Json<ErrorBody>) {
    match failure {
        ReplaceError::EnvironmentManaged => error(StatusCode::CONFLICT, "environment_managed"),
        ReplaceError::RevisionConflict => error(StatusCode::CONFLICT, "revision_conflict"),
        ReplaceError::Invalid => error(StatusCode::BAD_REQUEST, "invalid_settings"),
        ReplaceError::Persistence | ReplaceError::Internal => internal_error(),
    }
}

fn internal_error() -> (StatusCode, Json<ErrorBody>) {
    error(StatusCode::INTERNAL_SERVER_ERROR, "settings_unavailable")
}

fn error(status: StatusCode, code: &'static str) -> (StatusCode, Json<ErrorBody>) {
    (status, Json(ErrorBody { error: code }))
}
