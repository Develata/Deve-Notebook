//! plan_ref:
//!   - 15_settings#native-ai-provider-settings
//!   - 16_ai_agent#native-ai-chat-runtime

use crate::api::{
    AiProviderProtocol, AiProviderSettings, AiSettingsApiError, ReplaceAiProviderSettings,
};
use crate::i18n::{Locale, t};
use leptos::prelude::*;
use leptos::task::spawn_local;

const INPUT_CLASS: &str = "min-h-[44px] w-full rounded border border-default bg-panel px-3 py-2 text-sm text-primary outline-none focus:border-accent disabled:cursor-not-allowed disabled:opacity-60";
const BUTTON_CLASS: &str = "min-h-[44px] rounded bg-accent px-4 py-2 text-sm font-bold text-on-accent disabled:cursor-not-allowed disabled:opacity-50";

#[component]
pub fn AiProviderSettingsSection(locale: RwSignal<Locale>) -> impl IntoView {
    let (settings, set_settings) = signal(None::<AiProviderSettings>);
    let (provider, set_provider) = signal(AiProviderProtocol::OpenaiChatCompletions);
    let (base_url, set_base_url) = signal(String::new());
    let (model, set_model) = signal(String::new());
    let (max_tokens, set_max_tokens) = signal("4096".to_string());
    let (api_key, set_api_key) = signal(String::new());
    let (clear_key, set_clear_key) = signal(false);
    let (busy, set_busy) = signal(true);
    let (feedback, set_feedback) = signal(String::new());

    let apply_projection = move |projection: AiProviderSettings| {
        set_provider.set(projection.provider);
        set_base_url.set(projection.base_url.clone());
        set_model.set(projection.model.clone());
        set_max_tokens.set(projection.max_tokens.to_string());
        set_api_key.set(String::new());
        set_clear_key.set(false);
        set_settings.set(Some(projection));
    };

    spawn_local(async move {
        let result = crate::api::fetch_ai_provider_settings().await;
        if set_busy.try_set(false).is_some() {
            return;
        }
        match result {
            Ok(projection) => {
                apply_projection(projection);
                set_feedback.set(String::new());
            }
            Err(error) => set_feedback.set(api_error_copy(error, locale.get_untracked())),
        }
    });

    let save = move |_| {
        let Some(current) = settings.get_untracked() else {
            return;
        };
        let Ok(max_tokens) = max_tokens.get_untracked().trim().parse::<u32>() else {
            set_feedback.set(t::settings::ai_provider_invalid(locale.get_untracked()).to_string());
            return;
        };
        set_busy.set(true);
        set_feedback.set(t::settings::ai_provider_saving(locale.get_untracked()).to_string());
        let key = api_key.get_untracked();
        let request = ReplaceAiProviderSettings {
            expected_revision: current.revision,
            provider: provider.get_untracked(),
            base_url: base_url.get_untracked(),
            model: model.get_untracked(),
            max_tokens,
            api_key: (!key.is_empty()).then_some(key),
            clear_api_key: clear_key.get_untracked(),
        };
        spawn_local(async move {
            let result = crate::api::replace_ai_provider_settings(&request).await;
            if set_busy.try_set(false).is_some() {
                return;
            }
            match result {
                Ok(projection) => {
                    apply_projection(projection);
                    set_feedback
                        .set(t::settings::ai_provider_saved(locale.get_untracked()).to_string());
                }
                Err(error) => set_feedback.set(api_error_copy(error, locale.get_untracked())),
            }
        });
    };

    view! {
        <div
            tabindex="-1"
            class="bg-sidebar p-4 rounded-lg border border-default outline-none focus:border-accent"
            data-deve-settings-ai-provider="true"
            aria-labelledby="deve-ai-provider-settings-title"
            data-deve-ai-settings-source=move || settings.get().map(|value| value.source).unwrap_or_default()
        >
            <h3 id="deve-ai-provider-settings-title" class="font-medium text-primary">{move || t::settings::ai_provider_title(locale.get())}</h3>
            <p class="mt-1 text-xs text-muted">{move || t::settings::ai_provider_desc(locale.get())}</p>
            <Show when=move || settings.get().as_ref().is_some_and(|value| !value.writable)>
                <p class="mt-3 text-xs font-semibold text-accent" data-deve-ai-settings-readonly="true">
                    {move || t::settings::ai_provider_environment_managed(locale.get())}
                </p>
            </Show>
            <div class="mt-4 grid gap-3 sm:grid-cols-2">
                <label class="grid gap-1 text-xs text-muted">
                    <span>{move || t::settings::ai_provider_protocol(locale.get())}</span>
                    <select
                        class=INPUT_CLASS
                        prop:value=move || provider.get().as_str()
                        disabled=move || busy.get() || !settings.get().as_ref().is_some_and(|value| value.writable)
                        on:change=move |event| set_provider.set(AiProviderProtocol::parse(&event_target_value(&event)))
                        data-deve-ai-provider="true"
                    >
                        <option value="openai-chat-completions">"OpenAI Chat Completions"</option>
                        <option value="openai-responses">"OpenAI Responses"</option>
                        <option value="anthropic-messages">"Anthropic Messages"</option>
                    </select>
                </label>
                <label class="grid gap-1 text-xs text-muted">
                    <span>{move || t::settings::ai_provider_model(locale.get())}</span>
                    <input class=INPUT_CLASS prop:value=move || model.get() disabled=move || busy.get() || !settings.get().as_ref().is_some_and(|value| value.writable) on:input=move |event| set_model.set(event_target_value(&event)) data-deve-ai-model="true" />
                </label>
                <label class="grid gap-1 text-xs text-muted sm:col-span-2">
                    <span>{move || t::settings::ai_provider_base_url(locale.get())}</span>
                    <input class=INPUT_CLASS type="url" inputmode="url" prop:value=move || base_url.get() disabled=move || busy.get() || !settings.get().as_ref().is_some_and(|value| value.writable) on:input=move |event| set_base_url.set(event_target_value(&event)) data-deve-ai-base-url="true" />
                </label>
                <label class="grid gap-1 text-xs text-muted">
                    <span>{move || t::settings::ai_provider_max_tokens(locale.get())}</span>
                    <input class=INPUT_CLASS type="number" min="1" max="131072" prop:value=move || max_tokens.get() disabled=move || busy.get() || !settings.get().as_ref().is_some_and(|value| value.writable) on:input=move |event| set_max_tokens.set(event_target_value(&event)) data-deve-ai-max-tokens="true" />
                </label>
                <label class="grid gap-1 text-xs text-muted">
                    <span>{move || t::settings::ai_provider_api_key(locale.get())}</span>
                    <input class=INPUT_CLASS type="password" autocomplete="new-password" placeholder=move || if settings.get().as_ref().is_some_and(|value| value.key_configured) { t::settings::ai_provider_key_keep(locale.get()) } else { t::settings::ai_provider_key_missing(locale.get()) } prop:value=move || api_key.get() disabled=move || busy.get() || clear_key.get() || !settings.get().as_ref().is_some_and(|value| value.writable) on:input=move |event| set_api_key.set(event_target_value(&event)) data-deve-ai-api-key="true" />
                </label>
            </div>
            <div class="mt-4 flex flex-wrap gap-2">
                <button class=BUTTON_CLASS disabled=move || busy.get() || !settings.get().as_ref().is_some_and(|value| value.writable) on:click=save data-deve-ai-settings-save="true">
                    {move || t::settings::ai_provider_save(locale.get())}
                </button>
                <button class="min-h-[44px] rounded border border-default px-4 py-2 text-sm text-primary disabled:opacity-50" disabled=move || busy.get() || !settings.get().as_ref().is_some_and(|value| value.writable && value.key_configured) on:click=move |_| { set_api_key.set(String::new()); set_clear_key.update(|clear| *clear = !*clear); } data-deve-ai-settings-clear-key="true" data-deve-ai-settings-clear-pending=move || clear_key.get().to_string()>
                    {move || if clear_key.get() { t::settings::ai_provider_undo_clear_key(locale.get()) } else { t::settings::ai_provider_clear_key(locale.get()) }}
                </button>
            </div>
            <Show when=move || clear_key.get()>
                <p class="mt-3 text-xs font-semibold text-accent" role="status" aria-live="polite" data-deve-ai-settings-clear-notice="true">
                    {move || t::settings::ai_provider_clear_pending(locale.get())}
                </p>
            </Show>
            <p class="mt-3 text-xs text-muted" aria-live="polite" data-deve-ai-settings-feedback="true">{move || feedback.get()}</p>
        </div>
    }
}

fn api_error_copy(error: AiSettingsApiError, locale: Locale) -> String {
    match error {
        AiSettingsApiError::EnvironmentManaged => {
            t::settings::ai_provider_environment_managed(locale)
        }
        AiSettingsApiError::RevisionConflict => t::settings::ai_provider_revision_conflict(locale),
        AiSettingsApiError::Invalid => t::settings::ai_provider_invalid(locale),
        AiSettingsApiError::Unavailable => t::settings::ai_provider_unavailable(locale),
    }
    .to_string()
}
