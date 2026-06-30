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
    "min-h-11 md:min-h-0 px-2 py-1 md:py-0.5 flex justify-between items-center group cursor-pointer hover:bg-hover"
}

pub(super) fn icon_button_class(tone: SourceControlActionTone) -> String {
    format!(
        "h-11 w-11 md:h-auto md:w-auto p-0.5 hover:bg-active rounded flex items-center justify-center {}",
        match tone {
            SourceControlActionTone::Primary => "text-primary",
            SourceControlActionTone::Secondary => "text-secondary",
            SourceControlActionTone::Warning => "text-warning",
        }
    )
}

#[cfg(test)]
mod tests {
    use super::{
        SourceControlActionTone, change_item_row_class, icon_button_class, section_header_class,
    };

    #[test]
    fn mobile_source_control_read_gate_touch_targets_min_size_bound() {
        let row = change_item_row_class(false, true);
        assert!(row.contains("h-11"));
        assert!(row.contains("md:h-[22px]"));
        assert!(row.contains("cursor-pointer"));

        let section = section_header_class();
        assert!(section.contains("min-h-11"));
        assert!(section.contains("md:min-h-0"));

        for tone in [
            SourceControlActionTone::Primary,
            SourceControlActionTone::Secondary,
            SourceControlActionTone::Warning,
        ] {
            let button = icon_button_class(tone);
            assert!(button.contains("h-11"));
            assert!(button.contains("w-11"));
            assert!(button.contains("md:h-auto"));
            assert!(button.contains("md:w-auto"));
        }
    }
}
