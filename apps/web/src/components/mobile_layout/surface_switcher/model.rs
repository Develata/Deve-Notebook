//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-surface-switcher

use crate::components::editor_tabs::{EditorDiffTab, EditorDocumentTab, EditorTabKey};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct MobileSurfaceSummary {
    pub kind: &'static str,
    pub title: String,
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
                        title: tab.title.clone(),
                        total_count,
                    });
                }
            }
            EditorTabKey::Diff(key) => {
                if let Some(tab) = diff_tabs.iter().find(|tab| tab.key == key) {
                    return Some(MobileSurfaceSummary {
                        kind: "diff",
                        title: tab.title.clone(),
                        total_count,
                    });
                }
            }
        }
    }

    doc_tabs
        .first()
        .map(|tab| MobileSurfaceSummary {
            kind: "document",
            title: tab.title.clone(),
            total_count,
        })
        .or_else(|| {
            diff_tabs.first().map(|tab| MobileSurfaceSummary {
                kind: "diff",
                title: tab.title.clone(),
                total_count,
            })
        })
}

pub(crate) fn mobile_surface_sheet_visible(open: bool, drawer_open: bool, has_tabs: bool) -> bool {
    open && !drawer_open && has_tabs
}

pub(crate) fn mobile_surface_switcher_button_class() -> &'static str {
    "flex h-11 min-w-0 w-full items-center gap-2 rounded-md border border-default bg-editor px-3 text-left text-primary active:bg-hover"
}

pub(crate) fn mobile_surface_row_class(active: bool) -> &'static str {
    if active {
        "flex h-11 w-full items-center gap-3 rounded-md bg-active px-3 text-left text-primary"
    } else {
        "flex h-11 w-full items-center gap-3 rounded-md px-3 text-left text-secondary active:bg-hover"
    }
}

pub(crate) fn mobile_surface_close_button_class() -> &'static str {
    "flex h-11 min-w-[44px] items-center justify-center rounded-md text-muted active:bg-hover"
}

#[cfg(test)]
mod tests {
    use super::{
        mobile_surface_close_button_class, mobile_surface_row_class, mobile_surface_sheet_visible,
        mobile_surface_summary, mobile_surface_switcher_button_class,
    };
    use crate::components::editor_tabs::{EditorDocumentTab, EditorTabKey, diff_tab_from_session};
    use crate::hooks::use_core::diff_session::DiffSessionWire;
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
    fn mobile_surface_summary_falls_back_to_first_document() {
        let doc_id = DocId::from_u128(1);
        let docs = vec![EditorDocumentTab {
            doc_id,
            title: "a.md".into(),
            tooltip: "notes/a.md".into(),
        }];

        let summary = mobile_surface_summary(None, &docs, &[]).expect("summary");

        assert_eq!(summary.kind, "document");
        assert_eq!(summary.title, "a.md");
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
    fn mobile_surface_touch_targets_are_at_least_44px() {
        assert!(mobile_surface_switcher_button_class().contains("h-11"));
        assert!(mobile_surface_row_class(false).contains("h-11"));
        assert!(mobile_surface_row_class(true).contains("h-11"));
        assert!(mobile_surface_close_button_class().contains("h-11"));
        assert!(mobile_surface_close_button_class().contains("min-w-[44px]"));
    }
}
