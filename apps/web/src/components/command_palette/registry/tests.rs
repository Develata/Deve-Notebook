use super::create_static_commands;
use crate::components::activity_bar::SidebarView;
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
            None,
            None,
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
            None,
            None,
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
        let (is_mobile, _) = signal(false);
        let (_, set_mobile_visible) = signal(false);
        let (_, set_active_view) = signal(SidebarView::Explorer);
        let sidebar_control = SidebarControl {
            is_mobile,
            set_visible,
            set_mobile_visible,
            set_active_view,
        };
        let locale = RwSignal::new(Locale::En);
        let commands = create_static_commands(
            Locale::En,
            Callback::new(|_| {}),
            Callback::new(|_| {}),
            set_show,
            locale,
            None,
            Some(sidebar_control),
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
            None,
            None,
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

#[test]
fn settings_command_routes_to_settings_surface_and_closes_palette() {
    let owner = leptos::reactive::owner::Owner::new();

    owner.with(|| {
        let (show_palette, set_show_palette) = signal(true);
        let (settings_opened, set_settings_opened) = signal(false);
        let locale = RwSignal::new(Locale::En);
        let commands = create_static_commands(
            Locale::En,
            Callback::new(move |_| set_settings_opened.set(true)),
            Callback::new(|_| {}),
            set_show_palette,
            locale,
            None,
            None,
        );
        let command = commands
            .iter()
            .find(|command| command.id == "settings")
            .expect("settings command");

        assert_eq!(command.title, "Open Settings");
        assert_eq!(command.group, "Settings");
        assert!(command.shortcut.is_none());
        assert!(command.enabled_when.contains("browser-local"));

        command.action.run(());

        assert!(settings_opened.get_untracked());
        assert!(!show_palette.get_untracked());
    });
}
