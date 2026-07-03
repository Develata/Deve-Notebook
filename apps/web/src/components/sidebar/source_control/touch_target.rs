//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-interaction-design
//!   - 12_source_control_ui#source-control-vscode-reference-contract
//!
//! Shared mobile touch-target classes for the Source Control view.

#[derive(Clone, Copy)]
pub(super) enum SourceControlActionTone {
    Primary,
    Secondary,
    Warning,
}

pub(super) fn change_item_row_class(has_conflict: bool, can_open_diff: bool) -> String {
    format!(
        "flex items-center px-4 py-1 md:py-0.5 hover:bg-hover text-[13px] group h-11 md:h-[22px] {} {}",
        if has_conflict {
            "text-warning bg-warning/5"
        } else {
            "text-primary"
        },
        if can_open_diff {
            "cursor-pointer"
        } else {
            "cursor-help"
        }
    )
}

pub(super) fn section_header_class() -> &'static str {
    "h-11 md:h-auto px-2 py-1 md:py-0.5 flex justify-between items-center group cursor-pointer hover:bg-hover"
}

pub(super) fn secondary_panel_toggle_class() -> &'static str {
    "w-full h-11 md:h-auto flex items-center rounded-sm px-2 md:px-1 py-2 md:py-0.5 hover:bg-hover text-[11px] font-bold text-primary uppercase group focus:outline-none focus-visible:ring-1 focus-visible:ring-accent/40"
}

pub(super) fn header_container_class() -> &'static str {
    "flex-none h-11 md:h-9 flex items-center justify-between px-4 hover:bg-hover group border-b border-transparent hover:border-default relative"
}

pub(super) fn header_menu_trigger_class() -> &'static str {
    "h-11 w-11 md:h-5 md:w-5 p-0.5 hover:bg-active rounded flex items-center justify-center focus:outline-none focus-visible:ring-1 focus-visible:ring-accent/40"
}

pub(super) fn icon_button_class(tone: SourceControlActionTone) -> String {
    format!(
        "h-11 w-11 md:h-5 md:w-5 p-0.5 hover:bg-active rounded flex items-center justify-center {}",
        match tone {
            SourceControlActionTone::Primary => "text-primary",
            SourceControlActionTone::Secondary => "text-secondary",
            SourceControlActionTone::Warning => "text-warning",
        }
    )
}

pub(super) fn commit_message_textarea_class() -> &'static str {
    "w-full h-11 md:h-9 p-2 md:p-1.5 pr-12 md:pr-9 text-[13px] bg-input border border-default rounded-[2px] focus:outline-none focus:border-b-accent focus:ring-1 focus:ring-accent placeholder:text-muted text-primary font-sans resize-none block leading-tight"
}

pub(super) fn commit_generate_button_class() -> &'static str {
    "absolute right-0 md:right-1 top-0 md:top-1 bottom-0 md:bottom-1 w-11 md:w-7 bg-accent hover:bg-accent-hover text-on-accent rounded flex items-center justify-center transition-colors z-[calc(var(--z-editor)_+_1)] disabled:opacity-50 disabled:cursor-not-allowed"
}

pub(super) fn commit_primary_button_class(show_split: bool) -> String {
    format!(
        "flex-1 h-11 md:h-auto bg-accent hover:bg-accent-hover text-on-accent text-[13px] font-medium py-2 md:py-1.5 {} flex items-center justify-center gap-1 disabled:opacity-50 disabled:bg-accent disabled:cursor-not-allowed transition-colors shadow-sm",
        if show_split {
            "rounded-l-[2px]"
        } else {
            "rounded-[2px]"
        }
    )
}

pub(super) fn commit_dropdown_button_class() -> &'static str {
    "h-11 w-11 md:h-auto md:w-auto bg-accent hover:bg-accent-hover text-on-accent px-3 md:px-2 rounded-r-[2px] border-l border-white/20 flex items-center justify-center"
}

pub(super) fn commit_menu_item_class() -> &'static str {
    "w-full h-11 md:h-auto text-left px-3 py-2 md:py-1.5 hover:bg-hover text-primary flex items-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed"
}

#[cfg(test)]
mod tests {
    use super::{
        SourceControlActionTone, change_item_row_class, commit_dropdown_button_class,
        commit_generate_button_class, commit_menu_item_class, commit_message_textarea_class,
        commit_primary_button_class, header_container_class, header_menu_trigger_class,
        icon_button_class, secondary_panel_toggle_class, section_header_class,
    };

    #[test]
    fn mobile_source_control_read_gate_touch_targets_min_size_bound() {
        let row = change_item_row_class(false, true);
        assert!(row.contains("h-11"));
        assert!(row.contains("md:h-[22px]"));
        assert!(row.contains("cursor-pointer"));

        let section = section_header_class();
        assert!(section.contains("h-11"));
        assert!(section.contains("md:h-auto"));

        for tone in [
            SourceControlActionTone::Primary,
            SourceControlActionTone::Secondary,
            SourceControlActionTone::Warning,
        ] {
            let button = icon_button_class(tone);
            assert!(button.contains("h-11"));
            assert!(button.contains("w-11"));
            assert!(button.contains("md:h-5"));
            assert!(button.contains("md:w-5"));
        }
    }

    #[test]
    fn mobile_source_control_commit_touch_targets_min_size_bound() {
        let textarea = commit_message_textarea_class();
        assert!(textarea.contains("h-11"));
        assert!(textarea.contains("md:h-9"));

        let generate = commit_generate_button_class();
        assert!(generate.contains("w-11"));
        assert!(generate.contains("md:w-7"));

        let primary = commit_primary_button_class(true);
        assert!(primary.contains("h-11"));
        assert!(primary.contains("md:h-auto"));
        assert!(primary.contains("rounded-l-[2px]"));

        let primary_unsplit = commit_primary_button_class(false);
        assert!(primary_unsplit.contains("rounded-[2px]"));

        let dropdown = commit_dropdown_button_class();
        assert!(dropdown.contains("h-11"));
        assert!(dropdown.contains("w-11"));
        assert!(dropdown.contains("md:h-auto"));
        assert!(dropdown.contains("md:w-auto"));

        let menu_item = commit_menu_item_class();
        assert!(menu_item.contains("h-11"));
        assert!(menu_item.contains("md:h-auto"));
    }

    #[test]
    fn mobile_source_control_header_menu_trigger_is_at_least_44px() {
        let header = header_container_class();
        assert!(header.contains("h-11"));
        assert!(header.contains("md:h-9"));

        let trigger = header_menu_trigger_class();
        assert!(trigger.contains("h-11"));
        assert!(trigger.contains("w-11"));
        assert!(trigger.contains("md:h-5"));
        assert!(trigger.contains("md:w-5"));
    }

    #[test]
    fn mobile_source_control_secondary_panel_toggles_are_at_least_44px() {
        let toggle = secondary_panel_toggle_class();
        assert!(toggle.contains("h-11"));
        assert!(toggle.contains("md:h-auto"));
    }
}
