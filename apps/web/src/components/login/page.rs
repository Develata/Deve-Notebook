use crate::i18n::Locale;
use crate::i18n::login as login_i18n;
use leptos::prelude::*;
use leptos::task::spawn_local;

use super::api::{LoginAttemptError, attempt_login};
use super::state::AuthState;

#[component]
pub fn LoginPage(
    auth_state: ReadSignal<AuthState>,
    set_auth_state: WriteSignal<AuthState>,
) -> impl IntoView {
    let (username, set_username) = signal(String::new());
    let (password, set_password) = signal(String::new());
    let (is_loading, set_is_loading) = signal(false);
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));

    let error_message = move || match auth_state.get() {
        AuthState::Failed(msg) => Some(msg),
        _ => None,
    };

    let do_login = move |_: leptos::ev::MouseEvent| {
        let user = username.get();
        let pass = password.get();
        if user.is_empty() || pass.is_empty() {
            set_auth_state.set(AuthState::Failed(
                login_i18n::empty_fields(locale.get()).to_string(),
            ));
            return;
        }

        set_is_loading.set(true);
        set_auth_state.set(AuthState::Authenticating);

        spawn_local(async move {
            let locale_value = locale.get();
            let result = attempt_login(user, pass).await;
            set_is_loading.set(false);
            match result {
                Ok(()) => set_auth_state.set(AuthState::Authenticated),
                Err(err) => {
                    set_auth_state.set(AuthState::Failed(error_message_for(locale_value, err)))
                }
            }
        });
    };

    view! {
        <div class="fixed inset-0 bg-bg flex items-center justify-center z-50">
            <div class="w-full max-w-sm p-8 bg-bg-panel rounded-lg shadow-lg border border-border">
                <h1 class="text-2xl font-bold text-center text-primary mb-2">
                    {move || login_i18n::app_name(locale.get())}
                </h1>
                <p class="text-sm text-muted text-center mb-6">
                    {move || login_i18n::subtitle(locale.get())}
                </p>
                {move || error_message().map(|msg| view! {
                    <div class="mb-4 p-3 bg-red-900/20 border border-red-500/30 rounded text-red-400 text-sm">
                        {msg}
                    </div>
                })}
                <div class="space-y-4">
                    <div>
                        <label class="block text-xs font-medium text-muted mb-1">{move || login_i18n::username(locale.get())}</label>
                        <input
                            type="text"
                            class="w-full px-3 py-2 bg-bg border border-border rounded text-primary text-sm focus:outline-none focus:border-accent"
                            prop:value=move || username.get()
                            on:input=move |e| set_username.set(event_target_value(&e))
                            placeholder=move || login_i18n::username_placeholder(locale.get())
                        />
                    </div>
                    <div>
                        <label class="block text-xs font-medium text-muted mb-1">{move || login_i18n::password(locale.get())}</label>
                        <input
                            type="password"
                            class="w-full px-3 py-2 bg-bg border border-border rounded text-primary text-sm focus:outline-none focus:border-accent"
                            prop:value=move || password.get()
                            on:input=move |e| set_password.set(event_target_value(&e))
                            placeholder=move || login_i18n::password_placeholder(locale.get())
                        />
                    </div>
                    <button
                        class=move || format!(
                            "w-full py-2 px-4 rounded font-medium text-sm transition-colors {}",
                            if is_loading.get() { "bg-accent/50 cursor-not-allowed text-white" } else { "bg-accent hover:bg-accent/80 text-white" }
                        )
                        on:click=do_login
                    >
                        {move || if is_loading.get() { login_i18n::loading(locale.get()).to_string() } else { login_i18n::button(locale.get()).to_string() }}
                    </button>
                </div>
            </div>
        </div>
    }
}

fn error_message_for(locale: Locale, err: LoginAttemptError) -> String {
    match err {
        LoginAttemptError::Rejected(code) => login_i18n::auth_error(locale, code).to_string(),
        LoginAttemptError::InvalidResponse => format!(
            "{}: {}",
            login_i18n::transport_error(locale),
            login_i18n::invalid_response(locale)
        ),
        LoginAttemptError::Transport(message) => {
            format!("{}: {}", login_i18n::transport_error(locale), message)
        }
    }
}
