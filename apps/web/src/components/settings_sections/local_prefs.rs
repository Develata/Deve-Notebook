//! plan_ref:
//!   - 15_settings#browser-ui-prefs

use crate::components::layout_context::EditorTabLimitControl;
use crate::components::settings_prefs::{
    DEFAULT_MAX_DOCUMENT_TABS, EditorDensityPreference, EditorWrapPreference,
    MAX_MAX_DOCUMENT_TABS, MIN_MAX_DOCUMENT_TABS, ThemePreference, apply_editor_density_preference,
    apply_editor_wrap_preference, apply_theme_preference, clamp_max_document_tabs,
    persist_editor_density_preference, persist_editor_wrap_preference, persist_theme_preference,
    read_editor_density_preference, read_editor_wrap_preference, read_theme_preference,
};
use crate::components::settings_sections_policy::{
    editor_density_button_state, editor_wrap_button_state, theme_button_state,
};
use crate::i18n::{Locale, t};
use leptos::prelude::*;

fn submitted_max_document_tabs(raw: &str) -> usize {
    raw.trim()
        .parse::<usize>()
        .map(clamp_max_document_tabs)
        .unwrap_or(DEFAULT_MAX_DOCUMENT_TABS)
}

/// Browser-local theme preference.
#[component]
pub fn AppearanceSection(locale: RwSignal<Locale>) -> impl IntoView {
    let (theme_pref, set_theme_pref) = signal(read_theme_preference());
    Effect::new(move |_| apply_theme_preference(theme_pref.get()));
    let button_state = Signal::derive(move || theme_button_state(theme_pref.get()));

    view! {
        <div class="bg-sidebar p-4 rounded-lg border border-default">
            <div class="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between sm:gap-4">
                <div>
                    <h3 class="text-sm font-semibold text-muted uppercase tracking-wider mb-1">
                        {move || t::settings::appearance(locale.get())}
                    </h3>
                    <p class="text-xs text-muted leading-relaxed">
                        {move || t::settings::appearance_desc(locale.get())}
                    </p>
                </div>
                <div class="flex flex-wrap gap-2" data-deve-settings-theme=move || theme_pref.get().as_str()>
                    <button
                        class=move || button_state.get().auto_class
                        on:click=move |_| {
                            persist_theme_preference(ThemePreference::Auto);
                            set_theme_pref.set(ThemePreference::Auto);
                        }
                    >
                        {move || t::settings::theme_auto(locale.get())}
                    </button>
                    <button
                        class=move || button_state.get().light_class
                        on:click=move |_| {
                            persist_theme_preference(ThemePreference::Light);
                            set_theme_pref.set(ThemePreference::Light);
                        }
                    >
                        {move || t::settings::theme_light(locale.get())}
                    </button>
                    <button
                        class=move || button_state.get().dark_class
                        on:click=move |_| {
                            persist_theme_preference(ThemePreference::Dark);
                            set_theme_pref.set(ThemePreference::Dark);
                        }
                    >
                        {move || t::settings::theme_dark(locale.get())}
                    </button>
                </div>
            </div>
        </div>
    }
}

/// Browser-local editor preference markers.
#[component]
pub fn EditorBasicsSection(locale: RwSignal<Locale>) -> impl IntoView {
    let (wrap_pref, set_wrap_pref) = signal(read_editor_wrap_preference());
    let (density_pref, set_density_pref) = signal(read_editor_density_preference());
    let tab_limit = expect_context::<EditorTabLimitControl>();
    Effect::new(move |_| apply_editor_wrap_preference(wrap_pref.get()));
    Effect::new(move |_| apply_editor_density_preference(density_pref.get()));
    let wrap_state = Signal::derive(move || editor_wrap_button_state(wrap_pref.get()));
    let density_state = Signal::derive(move || editor_density_button_state(density_pref.get()));
    let max_document_tabs = tab_limit.max_document_tabs;
    let set_max_document_tabs = tab_limit.set_max_document_tabs;
    let (max_document_tabs_draft, set_max_document_tabs_draft) =
        signal(max_document_tabs.get_untracked().to_string());
    Effect::new(move |_| {
        let committed = max_document_tabs.get().to_string();
        if max_document_tabs_draft.get_untracked() != committed {
            set_max_document_tabs_draft.set(committed);
        }
    });
    let commit_max_document_tabs = Callback::new(move |raw: String| {
        let next = submitted_max_document_tabs(&raw);
        set_max_document_tabs.set(next);
        set_max_document_tabs_draft.set(next.to_string());
    });

    view! {
        <div class="bg-sidebar p-4 rounded-lg border border-default">
            <h3 class="text-sm font-semibold text-muted uppercase tracking-wider mb-1">
                {move || t::settings::editor_basics(locale.get())}
            </h3>
            <p class="text-xs text-muted leading-relaxed mb-4">
                {move || t::settings::editor_basics_desc(locale.get())}
            </p>
            <div class="space-y-3">
                <div class="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between sm:gap-4">
                    <span class="font-medium text-primary">{move || t::settings::word_wrap(locale.get())}</span>
                    <div class="flex flex-wrap gap-2" data-deve-settings-editor-wrap=move || wrap_pref.get().as_str()>
                        <button
                            class=move || wrap_state.get().on_class
                            on:click=move |_| {
                                persist_editor_wrap_preference(EditorWrapPreference::On);
                                set_wrap_pref.set(EditorWrapPreference::On);
                            }
                        >
                            {move || t::settings::word_wrap_on(locale.get())}
                        </button>
                        <button
                            class=move || wrap_state.get().off_class
                            on:click=move |_| {
                                persist_editor_wrap_preference(EditorWrapPreference::Off);
                                set_wrap_pref.set(EditorWrapPreference::Off);
                            }
                        >
                            {move || t::settings::word_wrap_off(locale.get())}
                        </button>
                    </div>
                </div>
                <div class="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between sm:gap-4">
                    <span class="font-medium text-primary">{move || t::settings::editor_density(locale.get())}</span>
                    <div class="flex flex-wrap gap-2" data-deve-settings-editor-density=move || density_pref.get().as_str()>
                        <button
                            class=move || density_state.get().comfortable_class
                            on:click=move |_| {
                                persist_editor_density_preference(EditorDensityPreference::Comfortable);
                                set_density_pref.set(EditorDensityPreference::Comfortable);
                            }
                        >
                            {move || t::settings::editor_density_comfortable(locale.get())}
                        </button>
                        <button
                            class=move || density_state.get().compact_class
                            on:click=move |_| {
                                persist_editor_density_preference(EditorDensityPreference::Compact);
                                set_density_pref.set(EditorDensityPreference::Compact);
                            }
                        >
                            {move || t::settings::editor_density_compact(locale.get())}
                        </button>
                    </div>
                </div>
                <div class="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between sm:gap-4">
                    <div class="min-w-0">
                        <label for="deve-settings-max-document-tabs" class="font-medium text-primary">
                            {move || t::settings::max_document_tabs(locale.get())}
                        </label>
                        <p class="text-xs text-muted">
                            {move || t::settings::max_document_tabs_desc(locale.get())}
                        </p>
                    </div>
                    <input
                        id="deve-settings-max-document-tabs"
                        type="number"
                        min=MIN_MAX_DOCUMENT_TABS.to_string()
                        max=MAX_MAX_DOCUMENT_TABS.to_string()
                        step="1"
                        class="min-h-[44px] w-24 rounded border border-default bg-panel px-3 py-2 text-sm text-primary"
                        title=move || t::settings::max_document_tabs_hint(locale.get())
                        aria-label=move || t::settings::max_document_tabs(locale.get())
                        data-deve-settings-max-document-tabs=move || max_document_tabs.get().to_string()
                        value=move || max_document_tabs_draft.get()
                        on:input=move |ev| set_max_document_tabs_draft.set(event_target_value(&ev))
                        on:change=move |ev| commit_max_document_tabs.run(event_target_value(&ev))
                        on:blur=move |_| commit_max_document_tabs.run(max_document_tabs_draft.get_untracked())
                        on:keydown=move |ev| {
                            if ev.key() == "Enter" {
                                commit_max_document_tabs.run(max_document_tabs_draft.get_untracked());
                            }
                        }
                    />
                </div>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::submitted_max_document_tabs;
    use crate::components::settings_prefs::{
        DEFAULT_MAX_DOCUMENT_TABS, MAX_MAX_DOCUMENT_TABS, MIN_MAX_DOCUMENT_TABS,
    };

    #[test]
    fn submitted_max_document_tabs_defaults_and_clamps() {
        assert_eq!(submitted_max_document_tabs(""), DEFAULT_MAX_DOCUMENT_TABS);
        assert_eq!(
            submitted_max_document_tabs("invalid"),
            DEFAULT_MAX_DOCUMENT_TABS
        );
        assert_eq!(submitted_max_document_tabs("0"), MIN_MAX_DOCUMENT_TABS);
        assert_eq!(submitted_max_document_tabs("99"), MAX_MAX_DOCUMENT_TABS);
        assert_eq!(submitted_max_document_tabs("12"), 12);
    }
}

/// Runtime smoke command surface for local diagnostics.
#[component]
pub fn RuntimeDiagnosticsSection(locale: RwSignal<Locale>) -> impl IntoView {
    view! {
        <div class="bg-sidebar p-4 rounded-lg border border-default" data-deve-runtime-diagnostics="true">
            <h3 class="text-sm font-semibold text-muted uppercase tracking-wider mb-1">
                {move || t::settings::runtime_diagnostics(locale.get())}
            </h3>
            <p class="text-xs text-muted leading-relaxed mb-3">
                {move || t::settings::runtime_diagnostics_desc(locale.get())}
            </p>
            <div class="space-y-2 text-xs">
                <div data-deve-runtime-smoke="embedded">
                    <div class="text-secondary mb-1">{move || t::settings::embedded_runtime(locale.get())}</div>
                    <code class="block overflow-x-auto whitespace-nowrap rounded border border-default bg-panel px-2 py-1 font-mono text-primary">
                        "scripts/smoke-web-release-build.sh"
                    </code>
                </div>
                <div data-deve-runtime-smoke="trunk">
                    <div class="text-secondary mb-1">{move || t::settings::trunk_runtime(locale.get())}</div>
                    <code class="block overflow-x-auto whitespace-nowrap rounded border border-default bg-panel px-2 py-1 font-mono text-primary">
                        "NO_COLOR=true trunk serve --address 127.0.0.1 --port 8080"
                    </code>
                </div>
            </div>
        </div>
    }
}
