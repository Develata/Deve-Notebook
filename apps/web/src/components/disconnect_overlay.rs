// apps/web/src/components/disconnect_overlay.rs
//! plan_ref:
//!   - 11_ui_design/02_desktop#desktop-native-adapter-contract
//!   - 11_ui_design/03_mobile#mobile-native-adapter-contract
//!   - 08_auth#unauthorized-disconnected-ui
//!
use crate::api::ConnectionStatus;
use crate::i18n::{Locale, t};
use leptos::prelude::*;
use leptos::task::spawn_local;

#[component]
pub fn DisconnectedOverlay(status: Signal<ConnectionStatus>) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));
    let (show_native_local_action, set_show_native_local_action) = signal(false);
    let (native_local_busy, set_native_local_busy) = signal(false);
    let (native_local_feedback, set_native_local_feedback) = signal(String::new());

    spawn_local(async move {
        let config = crate::api::get_native_backend_config().await;
        set_show_native_local_action.set(native_local_backend_action_available(
            config.available,
            &config.mode,
        ));
    });

    let switch_native_local = move |_| {
        if native_local_busy.get_untracked() || !show_native_local_action.get_untracked() {
            return;
        }
        set_native_local_busy.set(true);
        set_native_local_feedback
            .set(t::settings::local_backend_switching(locale.get_untracked()).to_string());
        spawn_local(async move {
            let config = crate::api::switch_native_backend_local().await;
            set_native_local_busy.set(false);
            if config.available {
                set_show_native_local_action.set(false);
                set_native_local_feedback
                    .set(t::settings::local_backend_saved(locale.get_untracked()).to_string());
            } else {
                set_native_local_feedback.set(config.error.unwrap_or_else(|| {
                    t::settings::native_backend_unavailable(locale.get_untracked()).to_string()
                }));
            }
        });
    };

    view! {
        {move || {
            let status = status.get();
            let current_locale = locale.get();
            if let Some((title, body)) = overlay_copy(current_locale, status) {
                view! {
                    <div
                        class="fixed inset-0 z-[var(--z-toast)] bg-panel/80 backdrop-blur-sm flex flex-col items-center justify-center"
                        role="alertdialog"
                        aria-live="assertive"
                        data-deve-disconnect-overlay="lockdown"
                        data-deve-editing-disabled="true"
                    >
                        <div class="bg-panel p-8 rounded-xl shadow-lg border border-default text-center">
                            <div class="text-4xl mb-4">"🔒"</div>
                            <h1 class="text-2xl font-bold text-primary mb-2">{title}</h1>
                            <p class="text-secondary mb-6">{body}</p>
                            <div class="w-full bg-active rounded-full h-2.5">
                              <div class="bg-accent h-2.5 rounded-full animate-pulse" style="width: 100%"></div>
                            </div>
                            <div class="mt-4 text-sm text-muted">
                                {format!(
                                    "{}: {}",
                                    t::common::status(current_locale),
                                    overlay_status_copy(current_locale, status)
                                )}
                            </div>
                            <Show when=move || show_native_local_action.get()>
                                <div class="mt-5 flex flex-col items-center gap-2">
                                    <button
                                        class="min-h-[44px] rounded bg-accent px-4 py-2 text-sm font-bold text-on-accent transition-colors disabled:cursor-not-allowed disabled:opacity-50"
                                        disabled=move || native_local_busy.get()
                                        on:click=switch_native_local
                                        data-deve-native-backend-use-local-overlay="true"
                                    >
                                        {move || t::settings::use_local_backend(locale.get())}
                                    </button>
                                    <p
                                        class="text-xs text-muted"
                                        data-deve-native-backend-use-local-feedback=move || {
                                            if native_local_busy.get() {
                                                "pending"
                                            } else if native_local_feedback.get().is_empty() {
                                                "idle"
                                            } else {
                                                "reported"
                                            }
                                        }
                                    >
                                        {move || native_local_feedback.get()}
                                    </p>
                                </div>
                            </Show>
                        </div>
                    </div>
                }.into_any()
            } else {
                view! {}.into_any()
            }
        }}
    }
}

fn overlay_copy(locale: Locale, status: ConnectionStatus) -> Option<(&'static str, &'static str)> {
    match status {
        ConnectionStatus::NativeBootstrapInvalid => Some((
            t::common::native_bootstrap_invalid_title(locale),
            t::common::native_bootstrap_invalid_body(locale),
        )),
        ConnectionStatus::NativeSessionPending => Some((
            t::common::native_session_pending_title(locale),
            t::common::native_session_pending_body(locale),
        )),
        ConnectionStatus::NativeServiceOffline => Some((
            t::common::native_service_offline_title(locale),
            t::common::native_service_offline_body(locale),
        )),
        ConnectionStatus::NativeReprobeRequired => Some((
            t::common::native_reprobe_required_title(locale),
            t::common::native_reprobe_required_body(locale),
        )),
        ConnectionStatus::Disconnected | ConnectionStatus::Connecting => Some((
            t::common::disconnected(locale),
            t::common::reconnecting(locale),
        )),
        ConnectionStatus::Unauthorized | ConnectionStatus::Connected => None,
    }
}

fn overlay_status_copy(locale: Locale, status: ConnectionStatus) -> &'static str {
    match status {
        ConnectionStatus::Disconnected => t::common::disconnected(locale),
        ConnectionStatus::Connecting => t::common::reconnecting(locale),
        ConnectionStatus::Unauthorized => t::bottom_bar::unauthorized(locale),
        ConnectionStatus::NativeBootstrapInvalid => t::bottom_bar::native_bootstrap_invalid(locale),
        ConnectionStatus::NativeSessionPending => t::bottom_bar::native_session_pending(locale),
        ConnectionStatus::NativeServiceOffline => t::bottom_bar::native_service_offline(locale),
        ConnectionStatus::NativeReprobeRequired => t::bottom_bar::native_reprobe_required(locale),
        ConnectionStatus::Connected => t::bottom_bar::ready(locale),
    }
}

fn native_local_backend_action_available(bridge_available: bool, mode: &str) -> bool {
    bridge_available && mode == "remote"
}

#[cfg(test)]
mod tests {
    use super::{native_local_backend_action_available, overlay_copy, overlay_status_copy};
    use crate::api::ConnectionStatus;
    use crate::i18n::{Locale, t};

    #[test]
    fn disconnected_lockdown_overlay_shows_reconnecting_text() {
        for status in [ConnectionStatus::Disconnected, ConnectionStatus::Connecting] {
            assert_eq!(
                overlay_copy(Locale::En, status).map(|(_, body)| body),
                Some("Reconnecting...")
            );
        }
    }

    #[test]
    fn disconnected_lockdown_overlay_hidden_for_connected_and_unauthorized() {
        assert_eq!(overlay_copy(Locale::En, ConnectionStatus::Connected), None);
        assert_eq!(
            overlay_copy(Locale::En, ConnectionStatus::Unauthorized),
            None
        );
    }

    #[test]
    fn disconnected_lockdown_status_line_is_localized() {
        assert_eq!(
            overlay_status_copy(Locale::Zh, ConnectionStatus::Disconnected),
            t::common::disconnected(Locale::Zh)
        );
        assert_eq!(
            overlay_status_copy(Locale::En, ConnectionStatus::NativeServiceOffline),
            t::bottom_bar::native_service_offline(Locale::En)
        );
    }

    #[test]
    fn native_local_backend_overlay_action_requires_native_remote_mode() {
        assert!(native_local_backend_action_available(true, "remote"));
        assert!(!native_local_backend_action_available(true, "local"));
        assert!(!native_local_backend_action_available(false, "remote"));
        assert!(!native_local_backend_action_available(true, "unexpected"));
    }
}
