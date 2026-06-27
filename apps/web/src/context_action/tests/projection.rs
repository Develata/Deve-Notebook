use super::*;

#[test]
fn file_tree_action_readonly_catalog_keeps_shell_local_open_only() {
    let ids = project_context_actions(file_tree_request(true, ContextActionTargetKind::AnyNode))
        .into_iter()
        .map(|action| action.descriptor.id)
        .collect::<Vec<_>>();

    assert_eq!(ids, vec![ContextActionId::OpenInNewWindow]);
}

#[test]
fn file_tree_action_target_projection_accepts_file_and_folder_nodes() {
    let file_len =
        project_context_actions(file_tree_request(false, ContextActionTargetKind::File)).len();
    let folder_len =
        project_context_actions(file_tree_request(false, ContextActionTargetKind::Folder)).len();
    let web_projectable_len = CONTEXT_ACTIONS
        .iter()
        .filter(|action| action.is_web_projectable())
        .count();

    assert_eq!(file_len, web_projectable_len);
    assert_eq!(folder_len, web_projectable_len);
}

#[test]
fn file_tree_action_projection_keeps_target_specific_descriptors() {
    let actions = [
        ContextActionDescriptor {
            id: ContextActionId::Rename,
            label: |_| "Any",
            icon: ContextActionIcon::Rename,
            origin: ContextActionOrigin::BackendNativeIntent,
            target_kind: ContextActionTargetKind::AnyNode,
            effect: ContextActionEffect::AuthorityWrite,
            readonly_allowed: false,
            separator_before: false,
            surfaces: TEST_SURFACE,
        },
        ContextActionDescriptor {
            id: ContextActionId::Copy,
            label: |_| "Markdown",
            icon: ContextActionIcon::Copy,
            origin: ContextActionOrigin::BackendNativeIntent,
            target_kind: ContextActionTargetKind::MarkdownFile,
            effect: ContextActionEffect::AuthorityWrite,
            readonly_allowed: false,
            separator_before: false,
            surfaces: TEST_SURFACE,
        },
        ContextActionDescriptor {
            id: ContextActionId::MoveTo,
            label: |_| "Folder",
            icon: ContextActionIcon::MoveTo,
            origin: ContextActionOrigin::BackendNativeIntent,
            target_kind: ContextActionTargetKind::Folder,
            effect: ContextActionEffect::AuthorityWrite,
            readonly_allowed: false,
            separator_before: false,
            surfaces: TEST_SURFACE,
        },
    ];

    let markdown_ids = project_context_actions_from_catalog(
        &actions,
        &file_tree_request(false, ContextActionTargetKind::MarkdownFile),
    )
    .into_iter()
    .map(|action| action.descriptor.id)
    .collect::<Vec<_>>();
    let folder_ids = project_context_actions_from_catalog(
        &actions,
        &file_tree_request(false, ContextActionTargetKind::Folder),
    )
    .into_iter()
    .map(|action| action.descriptor.id)
    .collect::<Vec<_>>();

    assert_eq!(
        markdown_ids,
        vec![ContextActionId::Rename, ContextActionId::Copy]
    );
    assert_eq!(
        folder_ids,
        vec![ContextActionId::Rename, ContextActionId::MoveTo]
    );
}

#[test]
fn file_tree_action_export_pdf_is_registered_but_default_unavailable() {
    let export = context_action_by_id(ContextActionId::ExportPdf).expect("export pdf action");
    let projected_ids = project_context_actions(file_tree_request(
        false,
        ContextActionTargetKind::MarkdownFile,
    ))
    .into_iter()
    .map(|action| action.descriptor.id)
    .collect::<Vec<_>>();

    assert_eq!(export.origin, ContextActionOrigin::ExternalProcess);
    assert_eq!(export.effect, ContextActionEffect::ExternalSideEffect);
    assert!(export.shows_external_provenance());
    assert!(!export.is_web_projectable());
    assert!(!projected_ids.contains(&ContextActionId::ExportPdf));
}

#[test]
fn file_tree_action_external_process_is_not_projected_by_web() {
    let actions = [
        ContextActionDescriptor {
            id: ContextActionId::OpenInNewWindow,
            label: |_| "External",
            icon: ContextActionIcon::OpenInNewWindow,
            origin: ContextActionOrigin::ExternalProcess,
            target_kind: ContextActionTargetKind::MarkdownFile,
            effect: ContextActionEffect::ExternalSideEffect,
            readonly_allowed: true,
            separator_before: false,
            surfaces: TEST_SURFACE,
        },
        ContextActionDescriptor {
            id: ContextActionId::Rename,
            label: |_| "Native",
            icon: ContextActionIcon::Rename,
            origin: ContextActionOrigin::BackendNativeIntent,
            target_kind: ContextActionTargetKind::MarkdownFile,
            effect: ContextActionEffect::AuthorityWrite,
            readonly_allowed: false,
            separator_before: false,
            surfaces: TEST_SURFACE,
        },
    ];

    let ids = project_context_actions_from_catalog(
        &actions,
        &file_tree_request(false, ContextActionTargetKind::MarkdownFile),
    )
    .into_iter()
    .map(|action| action.descriptor.id)
    .collect::<Vec<_>>();

    assert_eq!(ids, vec![ContextActionId::Rename]);
}

#[test]
fn context_action_readiness_write_gate_blocks_write_projection() {
    let ids = project_context_actions(file_tree_request_with_readiness(
        file_tree_readiness(false).with_write_blocked(true),
        ContextActionTargetKind::AnyNode,
    ))
    .into_iter()
    .map(|action| action.descriptor.id)
    .collect::<Vec<_>>();

    assert_eq!(ids, vec![ContextActionId::OpenInNewWindow]);
}

#[test]
fn file_tree_action_projection_rejects_internal_repo_segments() {
    let actions = project_context_actions(ContextActionProjectionRequest::new(
        ContextActionSurface::FileTree,
        ContextActionTarget::new(
            ContextActionTargetKind::MarkdownFile,
            "notes/.git/config.md",
        ),
        file_tree_readiness(false),
    ));

    assert!(actions.is_empty());
}

#[test]
fn file_tree_action_projected_intents_resolve_again() {
    let readiness = file_tree_readiness(false)
        .with_scope(ContextActionScope::new(Some("repo-a".to_string()), 7));
    let projected = project_context_actions(file_tree_request_with_readiness(
        readiness.clone(),
        ContextActionTargetKind::MarkdownFile,
    ));

    for action in projected {
        let resolved = resolve_context_action(ContextActionResolveRequest::new(
            action.intent.clone(),
            readiness.clone(),
        ))
        .expect("projected action should resolve");

        assert_eq!(resolved.descriptor.id, action.descriptor.id);
        assert_eq!(resolved.intent, action.intent);
        assert_eq!(resolved.intent.scope, readiness.scope);
    }
}
