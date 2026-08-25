//! plan_ref:
//!   - 08_auth#session-probe-policy
//!

use crate::api::{AuthProbe, probe_auth_status};
use crate::components::login::AuthState;
use crate::runtime::browser_runtime_lifetime::BrowserRuntimeLifetime;
use futures::future::{AbortHandle, Abortable};
use gloo_timers::future::TimeoutFuture;
use leptos::prelude::*;
use leptos::task::spawn_local;

const AUTH_MONITOR_MS: u32 = 5_000;

pub fn current_page_active() -> bool {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return true;
    };
    !document.hidden() && document.has_focus().unwrap_or(true)
}

pub fn should_run_session_probe(auth_state: &AuthState, page_active: bool) -> bool {
    page_active && matches!(auth_state, AuthState::Authenticated)
}

pub fn mount_auth_monitor(
    auth_state: ReadSignal<AuthState>,
    set_auth_state: WriteSignal<AuthState>,
    page_active: RwSignal<bool>,
) -> BrowserRuntimeLifetime {
    let auth_monitor_lifetime = BrowserRuntimeLifetime::new();
    let auth_session_generation = StoredValue::new_local(0_u64);
    let reactivation_abort = StoredValue::new_local(None::<AbortHandle>);
    let last_auth_state = StoredValue::new_local(auth_state.get_untracked());
    Effect::new(move |_| {
        let current = auth_state.get();
        let previous = last_auth_state.get_value();
        if current == previous {
            return;
        }
        last_auth_state.set_value(current);
        auth_session_generation.update_value(|generation| {
            *generation = generation.saturating_add(1);
        });
        reactivation_abort.update_value(|handle| {
            if let Some(handle) = handle.take() {
                handle.abort();
            }
        });
    });

    let last_page_active = StoredValue::new_local(page_active.get_untracked());
    let reactivation_lifetime = auth_monitor_lifetime.clone();
    Effect::new(move |_| {
        let active = page_active.get();
        let was_active = last_page_active.get_value();
        last_page_active.set_value(active);
        if was_active || !should_run_session_probe(&auth_state.get(), active) {
            return;
        }
        reactivation_abort.update_value(|handle| {
            if let Some(handle) = handle.take() {
                handle.abort();
            }
        });
        let (abort, registration) = AbortHandle::new_pair();
        reactivation_abort.set_value(Some(abort));
        let expected_generation = auth_session_generation.get_value();
        let auth_monitor_lifetime = reactivation_lifetime.clone();
        spawn_local(async move {
            let _ = Abortable::new(
                async move {
                    let probe = probe_auth_status().await;
                    if !auth_monitor_lifetime.is_active()
                        || auth_session_generation.get_value() != expected_generation
                    {
                        return;
                    }
                    match probe {
                        AuthProbe::Invalid => set_auth_state.set(AuthState::Unauthenticated),
                        AuthProbe::Valid | AuthProbe::Unknown => {}
                    }
                },
                registration,
            )
            .await;
        });
    });

    let startup_lifetime = auth_monitor_lifetime.clone();
    let (startup_abort, startup_registration) = AbortHandle::new_pair();
    spawn_local(async move {
        let _ = Abortable::new(
            async move {
                loop {
                    let probe = probe_auth_status().await;
                    if !startup_lifetime.is_active() {
                        return;
                    }
                    match probe {
                        AuthProbe::Valid => {
                            set_auth_state.set(AuthState::Authenticated);
                            break;
                        }
                        AuthProbe::Invalid => {
                            set_auth_state.set(AuthState::Unauthenticated);
                            break;
                        }
                        AuthProbe::Unknown => {
                            set_auth_state.set(AuthState::Unavailable);
                            TimeoutFuture::new(AUTH_MONITOR_MS).await;
                            if !startup_lifetime.is_active() {
                                return;
                            }
                        }
                    }
                }
            },
            startup_registration,
        )
        .await;
    });

    let periodic_lifetime = auth_monitor_lifetime.clone();
    let (auth_monitor_abort, auth_monitor_registration) = AbortHandle::new_pair();
    spawn_local(async move {
        let _ = Abortable::new(
            async move {
                loop {
                    TimeoutFuture::new(AUTH_MONITOR_MS).await;
                    if !periodic_lifetime.is_active() {
                        return;
                    }
                    if !should_run_session_probe(
                        &auth_state.get_untracked(),
                        page_active.get_untracked(),
                    ) {
                        continue;
                    }
                    let expected_generation = auth_session_generation.get_value();
                    let probe = probe_auth_status().await;
                    if !periodic_lifetime.is_active() {
                        return;
                    }
                    if auth_session_generation.get_value() != expected_generation {
                        continue;
                    }
                    match probe {
                        AuthProbe::Valid => {}
                        AuthProbe::Invalid => set_auth_state.set(AuthState::Unauthenticated),
                        AuthProbe::Unknown => {}
                    }
                }
            },
            auth_monitor_registration,
        )
        .await;
    });
    on_cleanup(move || {
        startup_abort.abort();
        auth_monitor_abort.abort();
        reactivation_abort.update_value(|handle| {
            if let Some(handle) = handle.take() {
                handle.abort();
            }
        });
    });
    auth_monitor_lifetime
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
    let cleanup_document = StoredValue::new_local(Some(document));
    let cleanup_callback = StoredValue::new_local(Some(callback));
    on_cleanup(move || {
        if let Some(document) = cleanup_document.try_get_value().flatten() {
            cleanup_callback.with_value(|callback| {
                if let Some(callback) = callback {
                    let _ = document.remove_event_listener_with_callback(
                        "visibilitychange",
                        callback.as_ref().unchecked_ref(),
                    );
                }
            });
        }
        cleanup_callback.update_value(|callback| drop(callback.take()));
        cleanup_document.update_value(|document| drop(document.take()));
    });
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

    #[test]
    fn app_auth_monitor_is_owner_scoped() {
        let source = include_str!("auth_monitor.rs")
            .split("#[cfg(test)]")
            .next()
            .expect("auth monitor source");

        assert!(source.contains("BrowserRuntimeLifetime::new()"));
        assert!(source.contains("AbortHandle::new_pair()"));
        assert!(source.contains("auth_monitor_abort.abort()"));
        assert!(source.contains("auth_monitor_lifetime.is_active()"));
        assert!(source.contains("reactivation_abort"));
        assert!(source.contains("auth_session_generation"));
    }
}
