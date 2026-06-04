use super::create_static_commands;
use crate::components::layout_context::SidebarControl;
use crate::i18n::Locale;
use leptos::prelude::*;

#[test]
fn acc_cmd_004b_static_commands_include_git_bridge_notices() {
    // CMD-004B: Git mirror palette entries remain CLI-only command notices.
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
        );
        let ids = commands
            .iter()
            .map(|command| command.id.as_str())
            .collect::<Vec<_>>();

        assert!(ids.contains(&"git_import_changes"));
        assert!(ids.contains(&"git_status"));
        assert!(ids.contains(&"git_mirror"));
        assert!(ids.contains(&"git_export_mirror"));
        assert!(ids.contains(&"git_push_mirror"));
        assert!(ids.contains(&"git_repair_mirror"));
    });
}

#[test]
fn acc_cmd_004c_static_commands_partition_reserved_surfaces() {
    // CMD-004C: reserved Source Control and AI entries stay unavailable.
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
        );

        for id in [
            "source_control_sync",
            "source_control_commit",
            "source_control_push",
            "ai_retry_last_request",
            "ai_switch_backend",
            "ai_switch_plan",
            "ai_switch_build",
        ] {
            let command = commands
                .iter()
                .find(|command| command.id == id)
                .unwrap_or_else(|| panic!("missing command {id}"));
            assert!(
                command.availability.is_unavailable(),
                "{id} must remain unavailable until a backend contract is wired"
            );
        }
    });
}

#[test]
fn static_commands_include_sidebar_toggle_when_control_is_available() {
    let owner = leptos::reactive::owner::Owner::new();

    owner.with(|| {
        let (_, set_show) = signal(false);
        let (visible, set_visible) = signal(true);
        provide_context(SidebarControl { set_visible });
        let locale = RwSignal::new(Locale::En);
        let commands = create_static_commands(
            Locale::En,
            Callback::new(|_| {}),
            Callback::new(|_| {}),
            set_show,
            locale,
        );
        let command = commands
            .iter()
            .find(|command| command.id == "toggle_sidebar")
            .expect("toggle sidebar command");

        command.action.run(());

        assert!(!visible.get_untracked());
    });
}

#[test]
fn static_commands_expose_group_shortcut_and_enabled_conditions() {
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
        );

        let open = commands
            .iter()
            .find(|command| command.id == "open")
            .expect("open command");
        assert_eq!(open.group, "Navigation");
        assert_eq!(open.shortcut.as_deref(), Some("Ctrl+P"));
        assert!(open.enabled_when.contains("search"));

        let settings = commands
            .iter()
            .find(|command| command.id == "settings")
            .expect("settings command");
        assert_eq!(settings.group, "Settings");
        assert!(settings.enabled_when.contains("browser-local"));
    });
}
