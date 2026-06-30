use super::*;
use crate::i18n::Locale;

#[test]
fn file_tree_action_registry_lives_outside_sidebar_menu() {
    assert!(module_path!().contains("context_action"));
    assert!(!module_path!().contains("sidebar_menu"));
}

#[test]
fn file_tree_action_catalog_preserves_existing_order() {
    let ids = CONTEXT_ACTIONS
        .iter()
        .filter(|action| action.is_web_projectable())
        .map(|action| action.id)
        .collect::<Vec<_>>();

    assert_eq!(
        ids,
        vec![
            ContextActionId::Rename,
            ContextActionId::Copy,
            ContextActionId::OpenInNewWindow,
            ContextActionId::CopyAbsolutePath,
            ContextActionId::RevealInSystemExplorer,
            ContextActionId::MoveTo,
            ContextActionId::Delete,
        ]
    );
}

#[test]
fn file_tree_action_stable_ids_are_surface_neutral() {
    let ids = [
        (ContextActionId::Rename, "file.rename"),
        (ContextActionId::Copy, "file.copy"),
        (ContextActionId::OpenInNewWindow, "file.open_in_new_window"),
        (ContextActionId::CopyAbsolutePath, "file.copy_absolute_path"),
        (
            ContextActionId::RevealInSystemExplorer,
            "file.reveal_in_system_explorer",
        ),
        (ContextActionId::MoveTo, "file.move_to"),
        (ContextActionId::Delete, "file.delete"),
        (ContextActionId::ExportPdf, "file.export_pdf"),
    ];

    for (action, expected) in ids {
        assert_eq!(action.stable_id(), expected);
        assert!(!action.stable_id().starts_with("file_tree."));
    }
}

#[test]
fn file_tree_action_lookup_returns_descriptor_metadata() {
    let open = context_action_by_id(ContextActionId::OpenInNewWindow).expect("open action");
    let delete = context_action_by_id(ContextActionId::Delete).expect("delete action");

    assert_eq!(open.effect, ContextActionEffect::ReadOnly);
    assert_eq!(open.target_kind, ContextActionTargetKind::MarkdownFile);
    assert!(open.readonly_allowed);
    assert_eq!(delete.effect, ContextActionEffect::DestructiveWrite);
    assert!(!delete.readonly_allowed);
}

#[test]
fn file_tree_action_write_catalog_keeps_destructive_metadata() {
    let delete = CONTEXT_ACTIONS
        .iter()
        .find(|action| action.id == ContextActionId::Delete)
        .expect("delete action");

    assert!(delete.is_destructive());
    assert!(delete.separator_before);
    assert_eq!(delete.effect, ContextActionEffect::DestructiveWrite);
}

#[test]
fn file_tree_action_descriptor_labels_remain_localized() {
    let rename = CONTEXT_ACTIONS
        .iter()
        .find(|action| action.id == ContextActionId::Rename)
        .expect("rename action");
    let copy_absolute_path =
        context_action_by_id(ContextActionId::CopyAbsolutePath).expect("copy path action");
    let reveal =
        context_action_by_id(ContextActionId::RevealInSystemExplorer).expect("reveal action");
    let export = context_action_by_id(ContextActionId::ExportPdf).expect("export pdf action");

    assert_eq!(rename.label(Locale::En), "Rename");
    assert_eq!(rename.label(Locale::Zh), "重命名");
    assert_eq!(copy_absolute_path.label(Locale::En), "Copy Absolute Path");
    assert_eq!(copy_absolute_path.label(Locale::Zh), "复制绝对路径");
    assert_eq!(reveal.label(Locale::En), "Show in System File Manager");
    assert_eq!(reveal.label(Locale::Zh), "在系统资源管理器中显示");
    assert_eq!(export.label(Locale::En), "Export PDF");
    assert_eq!(export.label(Locale::Zh), "导出 PDF");
}
