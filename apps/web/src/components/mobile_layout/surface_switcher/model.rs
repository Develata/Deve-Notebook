//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-surface-switcher

use crate::components::editor_tabs::{EditorDiffTab, EditorDocumentTab, EditorTabKey};
use crate::i18n::{Locale, t};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct MobileSurfaceSummary {
    pub kind: &'static str,
    pub title: Option<String>,
    pub total_count: usize,
}

pub(crate) fn mobile_surface_summary(
    active_tab: Option<EditorTabKey>,
    doc_tabs: &[EditorDocumentTab],
    diff_tabs: &[EditorDiffTab],
) -> Option<MobileSurfaceSummary> {
    let total_count = doc_tabs.len() + diff_tabs.len();
    if total_count == 0 {
        return None;
    }

    if let Some(active) = active_tab {
        match active {
            EditorTabKey::Document(doc_id) => {
                if let Some(tab) = doc_tabs.iter().find(|tab| tab.doc_id == doc_id) {
                    return Some(MobileSurfaceSummary {
                        kind: "document",
                        title: Some(tab.title.clone()),
                        total_count,
                    });
                }
            }
            EditorTabKey::Diff(key) => {
                if let Some(tab) = diff_tabs.iter().find(|tab| tab.key == key) {
                    return Some(MobileSurfaceSummary {
                        kind: "diff",
                        title: Some(tab.title.clone()),
                        total_count,
                    });
                }
            }
        }
    }

    Some(MobileSurfaceSummary {
        kind: "tabs",
        title: None,
        total_count,
    })
}

pub(crate) fn mobile_surface_sheet_visible(open: bool, drawer_open: bool, has_tabs: bool) -> bool {
    open && !drawer_open && has_tabs
}

pub(crate) fn mobile_surface_switcher_next_open(
    open: bool,
    drawer_open: bool,
    has_tabs: bool,
) -> bool {
    if drawer_open || !has_tabs {
        return false;
    }

    !open
}

pub(crate) fn mobile_surface_expanded_state(sheet_visible: bool) -> &'static str {
    if sheet_visible { "true" } else { "false" }
}

pub(crate) fn mobile_surface_switcher_button_class() -> &'static str {
    "flex h-11 min-w-0 w-full items-center gap-2 rounded-md border border-default bg-editor px-3 text-left text-primary active:bg-hover"
}

pub(crate) fn mobile_surface_summary_title_class(has_title: bool) -> &'static str {
    if has_title {
        "min-w-0 flex-1 truncate text-[13px] font-medium"
    } else {
        "min-w-0 flex-1 whitespace-nowrap text-[13px] font-medium"
    }
}

pub(crate) fn mobile_surface_summary_badge_text(
    kind: &str,
    total_count: usize,
    locale: Locale,
) -> String {
    let count_label = format!("{} {}", total_count, t::common::open_tabs(locale));
    match kind {
        "document" => format!("{} · {count_label}", t::common::document_tab(locale)),
        "diff" => format!("{} · {count_label}", t::common::diff_tab(locale)),
        _ => count_label,
    }
}

pub(crate) fn mobile_surface_row_class(active: bool) -> &'static str {
    if active {
        "flex h-11 min-w-0 flex-1 items-center gap-3 rounded-md bg-active px-3 text-left text-primary"
    } else {
        "flex h-11 min-w-0 flex-1 items-center gap-3 rounded-md px-3 text-left text-secondary active:bg-hover"
    }
}

pub(crate) fn mobile_surface_close_button_class() -> &'static str {
    "flex h-11 min-w-[44px] items-center justify-center rounded-md text-muted active:bg-hover"
}

pub(crate) fn mobile_surface_row_touch_target() -> &'static str {
    "mobile_surface_rows"
}

pub(crate) fn mobile_surface_switcher_touch_target() -> &'static str {
    "surface_switcher"
}

pub(crate) fn mobile_surface_type_label_marker() -> &'static str {
    "surface_type"
}

pub(crate) fn mobile_surface_close_touch_target() -> &'static str {
    "mobile_surface_close_buttons"
}

pub(crate) fn mobile_surface_current_state(active: bool) -> &'static str {
    if active { "true" } else { "false" }
}

#[cfg(test)]
mod tests {
    use super::{
        mobile_surface_close_button_class, mobile_surface_close_touch_target,
        mobile_surface_current_state, mobile_surface_expanded_state, mobile_surface_row_class,
        mobile_surface_row_touch_target, mobile_surface_sheet_visible, mobile_surface_summary,
        mobile_surface_summary_badge_text, mobile_surface_summary_title_class,
        mobile_surface_switcher_button_class, mobile_surface_switcher_next_open,
        mobile_surface_switcher_touch_target, mobile_surface_type_label_marker,
    };
    use crate::components::editor_tabs::{EditorDocumentTab, EditorTabKey, diff_tab_from_session};
    use crate::i18n::{Locale, t};
    use crate::runtime::source_control_client::diff_session::DiffSessionWire;
    use deve_core::models::DocId;

    #[test]
    fn mobile_surface_summary_prefers_active_diff() {
        let doc_id = DocId::from_u128(1);
        let diff = diff_tab_from_session(
            DiffSessionWire::new("notes/a.md".into(), "old".into(), "new".into())
                .with_doc_id(Some(doc_id)),
        );
        let docs = vec![EditorDocumentTab {
            doc_id,
            title: "a.md".into(),
            tooltip: "notes/a.md".into(),
        }];
        let summary =
            mobile_surface_summary(Some(EditorTabKey::Diff(diff.key.clone())), &docs, &[diff])
                .expect("summary");

        assert_eq!(summary.kind, "diff");
        assert_eq!(summary.total_count, 2);
    }

    #[test]
    fn mobile_surface_summary_uses_open_tabs_fallback_when_no_surface_is_active() {
        let doc_id = DocId::from_u128(1);
        let docs = vec![EditorDocumentTab {
            doc_id,
            title: "a.md".into(),
            tooltip: "notes/a.md".into(),
        }];

        let summary = mobile_surface_summary(None, &docs, &[]).expect("summary");

        assert_eq!(summary.kind, "tabs");
        assert_eq!(summary.title, None);
        assert_eq!(summary.total_count, 1);
    }

    #[test]
    fn mobile_surface_sheet_gate_closes_for_drawer_or_empty_tabs() {
        assert!(mobile_surface_sheet_visible(true, false, true));
        assert!(!mobile_surface_sheet_visible(true, true, true));
        assert!(!mobile_surface_sheet_visible(true, false, false));
        assert!(!mobile_surface_sheet_visible(false, false, true));
    }

    #[test]
    fn mobile_surface_switcher_toggle_respects_sheet_gate() {
        assert!(mobile_surface_switcher_next_open(false, false, true));
        assert!(!mobile_surface_switcher_next_open(true, false, true));
        assert!(!mobile_surface_switcher_next_open(false, true, true));
        assert!(!mobile_surface_switcher_next_open(false, false, false));
        assert!(!mobile_surface_switcher_next_open(true, true, false));
    }

    #[test]
    fn mobile_surface_exposes_expanded_state() {
        assert_eq!(mobile_surface_expanded_state(false), "false");
        assert_eq!(mobile_surface_expanded_state(true), "true");
    }

    #[test]
    fn mobile_surface_rows_expose_current_state() {
        assert_eq!(mobile_surface_current_state(false), "false");
        assert_eq!(mobile_surface_current_state(true), "true");
    }

    #[test]
    fn mobile_surface_touch_targets_are_at_least_44px() {
        assert!(mobile_surface_switcher_button_class().contains("h-11"));
        for class in [
            mobile_surface_row_class(false),
            mobile_surface_row_class(true),
        ] {
            assert!(class.contains("h-11"));
            assert!(class.contains("min-w-0"));
            assert!(class.contains("flex-1"));
            assert!(!class.contains("w-full"));
        }
    }

    #[test]
    fn mobile_surface_fallback_title_keeps_open_tabs_copy_visible() {
        assert!(mobile_surface_summary_title_class(false).contains("whitespace-nowrap"));
        assert!(!mobile_surface_summary_title_class(false).contains("truncate"));
        assert!(mobile_surface_summary_title_class(true).contains("truncate"));
    }

    #[test]
    fn mobile_surface_badge_text_shows_surface_type_and_count() {
        assert_eq!(
            mobile_surface_summary_badge_text("document", 2, Locale::Zh),
            format!(
                "{} · 2 {}",
                t::common::document_tab(Locale::Zh),
                t::common::open_tabs(Locale::Zh)
            )
        );
        assert_eq!(
            mobile_surface_summary_badge_text("diff", 1, Locale::En),
            "Diff tab · 1 Open tabs"
        );
        assert_eq!(
            mobile_surface_summary_badge_text("tabs", 3, Locale::Zh),
            format!("3 {}", t::common::open_tabs(Locale::Zh))
        );
    }

    #[test]
    fn mobile_surface_badge_text_uses_tabs_page_copy_in_zh() {
        assert_eq!(
            mobile_surface_summary_badge_text("document", 2, Locale::Zh),
            format!(
                "{} · 2 {}",
                t::common::document_tab(Locale::Zh),
                t::common::open_tabs(Locale::Zh)
            )
        );
        assert_eq!(
            mobile_surface_summary_badge_text("tabs", 3, Locale::Zh),
            format!("3 {}", t::common::open_tabs(Locale::Zh))
        );
    }

    #[test]
    fn mobile_surface_close_buttons_are_at_least_44px() {
        let class = mobile_surface_close_button_class();
        assert!(class.contains("h-11"));
        assert!(class.contains("min-w-[44px]"));
    }

    #[test]
    fn mobile_surface_touch_target_markers_are_stable() {
        assert_eq!(mobile_surface_switcher_touch_target(), "surface_switcher");
        assert_eq!(mobile_surface_type_label_marker(), "surface_type");
        assert_eq!(mobile_surface_row_touch_target(), "mobile_surface_rows");
        assert_eq!(
            mobile_surface_close_touch_target(),
            "mobile_surface_close_buttons"
        );
    }
}
