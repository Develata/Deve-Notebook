use super::*;

#[test]
fn file_tree_action_export_pdf_default_not_resolved() {
    let resolved = resolve_context_action(ContextActionResolveRequest::new(
        file_tree_intent(
            ContextActionId::ExportPdf,
            ContextActionTargetKind::MarkdownFile,
        ),
        false,
    ));

    assert!(resolved.is_none());
}

#[test]
fn file_tree_action_resolver_blocks_writes_in_readonly_state() {
    for action_id in [
        ContextActionId::Rename,
        ContextActionId::Copy,
        ContextActionId::MoveTo,
        ContextActionId::Delete,
    ] {
        let resolved = resolve_context_action(ContextActionResolveRequest::new(
            file_tree_intent(action_id, ContextActionTargetKind::AnyNode),
            true,
        ));

        assert!(
            resolved.is_none(),
            "{} should not resolve",
            action_id.stable_id()
        );
    }

    let open = resolve_context_action(ContextActionResolveRequest::new(
        file_tree_intent(
            ContextActionId::OpenInNewWindow,
            ContextActionTargetKind::AnyNode,
        ),
        true,
    ))
    .expect("open in new window should resolve in readonly");

    assert_eq!(open.descriptor.id, ContextActionId::OpenInNewWindow);
}

#[test]
fn file_tree_action_resolver_rejects_surface_mismatch() {
    let intent = ContextActionIntent::new(
        ContextActionId::Rename,
        ContextActionSurface::CommandPalette,
        ContextActionTarget::new(ContextActionTargetKind::AnyNode, "notes/readme.md"),
    );

    assert!(resolve_context_action(ContextActionResolveRequest::new(intent, false)).is_none());
}

#[test]
fn file_tree_action_resolver_rejects_target_mismatch() {
    let actions = [ContextActionDescriptor {
        id: ContextActionId::Copy,
        label: |_| "Markdown only",
        icon: ContextActionIcon::Copy,
        origin: ContextActionOrigin::BackendNativeIntent,
        target_kind: ContextActionTargetKind::MarkdownFile,
        effect: ContextActionEffect::AuthorityWrite,
        readonly_allowed: false,
        separator_before: false,
        surfaces: TEST_SURFACE,
    }];

    let file_resolved = resolve_context_action_from_catalog(
        &actions,
        ContextActionResolveRequest::new(
            file_tree_intent(ContextActionId::Copy, ContextActionTargetKind::File),
            false,
        ),
    );
    let markdown_resolved = resolve_context_action_from_catalog(
        &actions,
        ContextActionResolveRequest::new(
            file_tree_intent(ContextActionId::Copy, ContextActionTargetKind::MarkdownFile),
            false,
        ),
    )
    .expect("markdown target should resolve");

    assert!(file_resolved.is_none());
    assert_eq!(markdown_resolved.descriptor.id, ContextActionId::Copy);
}
