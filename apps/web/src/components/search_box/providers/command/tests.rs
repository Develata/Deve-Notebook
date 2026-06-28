use super::CommandProvider;
use crate::components::command_palette::Command;
use crate::components::command_palette::registry::create_static_commands;
use crate::components::search_box::types::SearchProvider;
use crate::i18n::{Locale, t};
use leptos::prelude::{Callback, RwSignal, signal};

fn command(id: &str, title: &str) -> Command {
    Command::available(id, title, Callback::new(|_| {}))
}

fn unavailable_command(id: &str, title: &str, reason: &str) -> Command {
    Command::unavailable(id, title, reason, Callback::new(|_| {}))
}

#[test]
fn command_provider_matches_stable_command_id_words() {
    let provider = CommandProvider::new(
        vec![command(
            "git_import_changes",
            t::command_palette::git_import_changes(Locale::Zh),
        )],
        Locale::Zh,
    );

    let results = provider.search(">git import");

    assert_eq!(
        results.first().map(|result| result.id.as_str()),
        Some("git_import_changes")
    );
}

#[test]
fn command_provider_matches_command_id_even_when_title_is_localized() {
    let provider = CommandProvider::new(
        vec![command(
            "git_push_mirror",
            t::command_palette::git_push_mirror(Locale::Zh),
        )],
        Locale::Zh,
    );

    let results = provider.search(">git push");

    assert_eq!(
        results.first().map(|result| result.id.as_str()),
        Some("git_push_mirror")
    );
}

#[test]
fn command_provider_exposes_unavailable_reason_as_detail() {
    let provider = CommandProvider::new(
        vec![unavailable_command(
            "establish_branch",
            "P2P: Establish Branch",
            "Unavailable: no branch creation backend",
        )],
        Locale::En,
    );

    let results = provider.search(">establish");

    let detail = results
        .first()
        .and_then(|result| result.detail.as_deref())
        .expect("command detail");
    assert!(detail.contains("Unavailable: no branch creation backend"));
}

#[test]
fn command_provider_matches_visible_unavailable_reason() {
    let provider = CommandProvider::new(
        vec![unavailable_command(
            "establish_branch",
            "P2P: Establish Branch",
            "Unavailable: no branch creation backend",
        )],
        Locale::En,
    );

    let results = provider.search(">branch creation backend");

    assert_eq!(
        results.first().map(|result| result.id.as_str()),
        Some("establish_branch")
    );
}

#[test]
fn command_provider_detail_includes_group_shortcut_and_enabled_condition() {
    let provider = CommandProvider::new(
        vec![
            command("open", "Open Document")
                .with_group("Navigation")
                .with_shortcut("Ctrl+P")
                .with_enabled_when("Opens the current search surface"),
        ],
        Locale::En,
    );

    let results = provider.search(">navigation");
    let detail = results
        .first()
        .and_then(|result| result.detail.as_deref())
        .expect("command detail");

    assert!(detail.contains("Navigation"));
    assert!(detail.contains("Ctrl+P"));
    assert!(detail.contains("search surface"));
}

#[test]
fn command_provider_uses_registry_metadata_for_settings_entry() {
    let owner = leptos::reactive::owner::Owner::new();

    owner.with(|| {
        let (_, set_show) = signal(false);
        let locale = RwSignal::new(Locale::En);
        let commands = create_static_commands(
            Locale::En,
            Callback::new(|_| {}),
            Callback::new(|_| {}),
            set_show,
            locale,
            None,
            None,
        );
        let provider = CommandProvider::new(commands, Locale::En);

        let results = provider.search(">settings");
        let settings = results
            .iter()
            .find(|result| result.id == "settings")
            .expect("settings command result");
        let detail = settings.detail.as_deref().expect("settings detail");

        assert_eq!(
            settings.title,
            t::command_palette::open_settings(Locale::En)
        );
        assert!(detail.contains(t::command_palette::group_settings(Locale::En)));
        assert!(detail.contains("browser-local"));
        assert!(detail.contains("runtime feedback"));
    });
}
