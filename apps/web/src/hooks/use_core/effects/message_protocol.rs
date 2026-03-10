use crate::api::WsService;
use crate::i18n::{Locale, t};
use deve_core::protocol::{ServerError, ServerErrorCode};

pub fn handle_protocol_error(ws: &WsService, locale: Locale, error: &ServerError) {
    if is_auth_error(error.code) {
        ws.mark_unauthorized();
    }
    let message = t::server_error::message(locale, error.code);
    match error.detail.as_deref() {
        Some(detail) => leptos::logging::warn!("协议错误 {}: {}", message, detail),
        None => leptos::logging::warn!("协议错误 {}", message),
    }
    if let Some(window) = web_sys::window() {
        let _ = window.alert_with_message(message);
    }
}

fn is_auth_error(code: ServerErrorCode) -> bool {
    matches!(
        code,
        ServerErrorCode::AuthTokenExpired | ServerErrorCode::AuthTokenMissing
    )
}
