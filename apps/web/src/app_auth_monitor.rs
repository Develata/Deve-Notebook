use crate::components::login::AuthState;
use leptos::prelude::{RwSignal, Set};

pub fn current_page_active() -> bool {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return true;
    };
    !document.hidden() && document.has_focus().unwrap_or(true)
}

pub fn should_run_session_probe(auth_state: &AuthState, page_active: bool) -> bool {
    page_active && matches!(auth_state, AuthState::Authenticated)
}

pub fn mount_visibility_listener(page_active: RwSignal<bool>) {
    use wasm_bindgen::{JsCast, closure::Closure};

    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let callback = Closure::wrap(Box::new(move |_event: web_sys::Event| {
        page_active.set(current_page_active());
    }) as Box<dyn FnMut(_)>);
    let _ = document
        .add_event_listener_with_callback("visibilitychange", callback.as_ref().unchecked_ref());
    // App mounts once per page load, so a single leaked listener is acceptable here.
    callback.forget();
}

#[cfg(test)]
mod tests {
    use super::should_run_session_probe;
    use crate::components::login::AuthState;

    #[test]
    fn skips_session_probe_when_page_is_inactive() {
        assert!(!should_run_session_probe(&AuthState::Authenticated, false));
    }

    #[test]
    fn only_runs_session_probe_for_authenticated_sessions() {
        assert!(should_run_session_probe(&AuthState::Authenticated, true));
        assert!(!should_run_session_probe(&AuthState::Checking, true));
        assert!(!should_run_session_probe(&AuthState::Unavailable, true));
    }
}
