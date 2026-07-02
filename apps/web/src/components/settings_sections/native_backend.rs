//! plan_ref:
//!   - 15_settings#native-host-local-backend-preference

use crate::components::settings_sections_policy::{
    native_backend_button_state, native_backend_can_switch_local,
    native_backend_unavailable_feedback, native_backend_validation_state,
};
use crate::i18n::{Locale, t};
use leptos::prelude::*;
use leptos::task::spawn_local;

/// Native host-local backend selector. In regular browsers this section is
/// visible but unavailable; native shells provide the invoke bridge.
#[component]
pub fn NativeBackendSection(locale: RwSignal<Locale>) -> impl IntoView {
    let (available, set_available) = signal(false);
    let (mode, set_mode) = signal("local".to_string());
    let (remote_draft, set_remote_draft) = signal(String::new());
    let (feedback, set_feedback) = signal(String::new());
    let (remote_validation_succeeded, set_remote_validation_succeeded) = signal(false);
    let (busy, set_busy) = signal(false);

    spawn_local(async move {
        let config: crate::api::NativeBackendConfig = crate::api::get_native_backend_config().await;
        set_available.set(config.available);
        if config.available {
            set_mode.set(config.mode);
            set_remote_draft.set(config.remote_url);
            set_remote_validation_succeeded.set(false);
            set_feedback.set(String::new());
        } else {
            set_remote_validation_succeeded.set(false);
            set_feedback.set(config.error_message.unwrap_or_default());
        }
    });

    let button_state = Signal::derive(move || native_backend_button_state(&mode.get()));
    let save_remote = move |_| {
        if busy.get_untracked() {
            return;
        }
        let draft = remote_draft.get_untracked();
        if draft.trim().is_empty() {
            set_feedback.set(
                t::settings::remote_backend_requires_validation(locale.get_untracked()).to_string(),
            );
            return;
        }
        set_busy.set(true);
        set_remote_validation_succeeded.set(false);
        set_feedback
            .set(t::settings::validating_remote_backend(locale.get_untracked()).to_string());
        spawn_local(async move {
            let result: crate::api::NativeBackendValidation =
                crate::api::save_native_backend_remote(draft).await;
            set_busy.set(false);
            if !result.available {
                set_available.set(false);
                set_remote_validation_succeeded.set(false);
                set_feedback.set(result.error_message.unwrap_or_default());
                return;
            }
            if result.ok {
                set_available.set(true);
                set_mode.set("remote".to_string());
                set_remote_draft.set(result.https_origin);
                set_remote_validation_succeeded.set(true);
                set_feedback
                    .set(t::settings::remote_backend_saved(locale.get_untracked()).to_string());
            } else {
                set_remote_validation_succeeded.set(false);
                set_feedback.set(result.error_message.unwrap_or_else(|| {
                    t::settings::remote_backend_requires_validation(locale.get_untracked())
                        .to_string()
                }));
            }
        });
    };

    let switch_local = move |_| {
        if busy.get_untracked() {
            return;
        }
        if !native_backend_can_switch_local(&mode.get_untracked()) {
            return;
        }
        set_busy.set(true);
        set_remote_validation_succeeded.set(false);
        set_feedback.set(t::settings::local_backend_switching(locale.get_untracked()).to_string());
        spawn_local(async move {
            let config: crate::api::NativeBackendConfig =
                crate::api::switch_native_backend_local().await;
            set_busy.set(false);
            if config.available {
                set_available.set(true);
                set_mode.set("local".to_string());
                set_remote_draft.set(config.remote_url);
                set_remote_validation_succeeded.set(false);
                set_feedback
                    .set(t::settings::local_backend_saved(locale.get_untracked()).to_string());
            } else {
                set_available.set(false);
                set_remote_validation_succeeded.set(false);
                set_feedback.set(config.error_message.unwrap_or_default());
            }
        });
    };

    view! {
        <div
            class="bg-sidebar p-4 rounded-lg border border-default"
            data-deve-settings-native-backend="true"
            data-deve-native-backend-mode=move || mode.get()
            data-deve-native-backend-unavailable=move || (!available.get()).to_string()
        >
            <div class="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
                <div>
                    <span class="font-medium text-primary">{move || t::settings::backend_section(locale.get())}</span>
                    <p class="text-xs text-muted">{move || t::settings::backend_section_desc(locale.get())}</p>
                </div>
                <div class="flex flex-wrap gap-2">
                    <button
                        class=move || button_state.get().local_class
                        disabled=move || !available.get() || busy.get()
                        on:click=switch_local
                    >
                        {move || t::settings::local_backend_label(locale.get())}
                    </button>
                    <button
                        class=move || button_state.get().remote_class
                        disabled=move || !available.get() || busy.get()
                        on:click=move |_| {
                            set_remote_validation_succeeded.set(false);
                            set_feedback.set(String::new());
                            set_mode.set("remote".to_string());
                        }
                    >
                        {move || t::settings::remote_backend_label(locale.get())}
                    </button>
                </div>
            </div>

            <Show when=move || !available.get()>
                <p
                    class="mt-3 text-xs text-accent"
                    data-deve-native-backend-unavailable="true"
                >
                    {move || {
                        let text = feedback.get();
                        native_backend_unavailable_feedback(locale.get(), &text)
                    }}
                </p>
            </Show>

            <Show when=move || available.get()>
                <div class="mt-4 grid gap-3">
                    <label class="grid gap-1 text-xs text-muted">
                        <span>{move || t::settings::remote_backend_url_label(locale.get())}</span>
                        <input
                            class="min-h-[44px] rounded border border-default bg-panel px-3 py-2 text-sm text-primary outline-none focus:border-accent"
                            type="url"
                            inputmode="url"
                            placeholder="https://deve.example"
                            prop:value=move || remote_draft.get()
                            on:input=move |ev| {
                                set_remote_validation_succeeded.set(false);
                                set_feedback.set(String::new());
                                set_remote_draft.set(event_target_value(&ev));
                            }
                            disabled=move || busy.get()
                            data-deve-native-backend-remote-url="true"
                        />
                    </label>
                    <div class="flex flex-wrap gap-2">
                        <button
                            class="min-h-[44px] px-3 py-2 text-sm font-bold bg-accent text-on-accent rounded transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                            disabled=move || busy.get() || remote_draft.get().trim().is_empty()
                            on:click=save_remote
                            data-deve-native-backend-save-remote="true"
                        >
                            {move || t::settings::validate_and_save_remote(locale.get())}
                        </button>
                        <button
                            class="min-h-[44px] px-3 py-2 text-sm font-medium text-muted hover:bg-active rounded transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                            disabled=move || busy.get()
                            on:click=switch_local
                            data-deve-native-backend-use-local="true"
                        >
                            {move || t::settings::use_local_backend(locale.get())}
                        </button>
                    </div>
                    <p
                        class="text-xs text-muted"
                        data-deve-native-backend-validation=move || {
                            let feedback = feedback.get();
                            native_backend_validation_state(
                                busy.get(),
                                &feedback,
                                &mode.get(),
                                remote_validation_succeeded.get(),
                            )
                            .to_string()
                        }
                    >
                        {move || {
                            let text = feedback.get();
                            if text.is_empty() {
                                t::settings::remote_backend_requires_validation(locale.get()).to_string()
                            } else {
                                text
                            }
                        }}
                    </p>
                </div>
            </Show>
        </div>
    }
}
