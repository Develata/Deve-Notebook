use super::*;

#[test]
fn file_tree_action_export_pdf_default_not_resolved() {
    let resolved = resolve_context_action(ContextActionResolveRequest::new(
        file_tree_intent(
            ContextActionId::ExportPdf,
            ContextActionTargetKind::MarkdownFile,
        ),
        file_tree_readiness(false),
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
            file_tree_readiness(true),
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
        file_tree_readiness(true),
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

    assert!(
        resolve_context_action(ContextActionResolveRequest::new(
            intent,
            file_tree_readiness(false)
        ))
        .is_none()
    );
}

#[test]
fn context_action_readiness_scope_mismatch_not_resolved() {
    let projected_scope = ContextActionScope::new(Some("repo-a".to_string()), 1);
    let current_scope = ContextActionScope::new(Some("repo-a".to_string()), 2);
    let intent = file_tree_intent_with_scope(
        ContextActionId::Rename,
        ContextActionTargetKind::AnyNode,
        projected_scope,
    );
    let readiness = ContextActionReadiness::new(current_scope, false, false);

    assert!(resolve_context_action(ContextActionResolveRequest::new(intent, readiness)).is_none());
}

#[test]
fn context_action_readiness_write_gate_blocks_write_actions() {
    let readiness = file_tree_readiness(false).with_write_blocked(true);

    for action_id in [
        ContextActionId::Rename,
        ContextActionId::Copy,
        ContextActionId::MoveTo,
        ContextActionId::Delete,
    ] {
        let resolved = resolve_context_action(ContextActionResolveRequest::new(
            file_tree_intent(action_id, ContextActionTargetKind::AnyNode),
            readiness.clone(),
        ));

        assert!(
            resolved.is_none(),
            "{} should not resolve",
            action_id.stable_id()
        );
    }
}

#[test]
fn context_action_readiness_write_gate_keeps_readonly_action_available() {
    let resolved = resolve_context_action(ContextActionResolveRequest::new(
        file_tree_intent(
            ContextActionId::OpenInNewWindow,
            ContextActionTargetKind::AnyNode,
        ),
        file_tree_readiness(false).with_write_blocked(true),
    ))
    .expect("open in new window should resolve when write gate is blocked");

    assert_eq!(resolved.descriptor.id, ContextActionId::OpenInNewWindow);
}

#[test]
fn file_tree_action_host_actions_require_runtime_capability() {
    for action_id in [
        ContextActionId::CopyAbsolutePath,
        ContextActionId::RevealInSystemExplorer,
    ] {
        let resolved = resolve_context_action(ContextActionResolveRequest::new(
            file_tree_intent(action_id, ContextActionTargetKind::AnyNode),
            file_tree_readiness(true),
        ));

        assert!(
            resolved.is_none(),
            "{} should fail closed",
            action_id.stable_id()
        );
    }

    let copy = resolve_context_action(ContextActionResolveRequest::new(
        file_tree_intent(
            ContextActionId::CopyAbsolutePath,
            ContextActionTargetKind::AnyNode,
        ),
        file_tree_readiness(true).with_host_file_actions(true, false),
    ))
    .expect("copy absolute path should resolve when supported");
    let reveal = resolve_context_action(ContextActionResolveRequest::new(
        file_tree_intent(
            ContextActionId::RevealInSystemExplorer,
            ContextActionTargetKind::AnyNode,
        ),
        file_tree_readiness(true).with_host_file_actions(true, true),
    ))
    .expect("reveal should resolve when supported");

    assert_eq!(copy.descriptor.id, ContextActionId::CopyAbsolutePath);
    assert_eq!(
        reveal.descriptor.id,
        ContextActionId::RevealInSystemExplorer
    );
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
            file_tree_readiness(false),
        ),
    );
    let markdown_resolved = resolve_context_action_from_catalog(
        &actions,
        ContextActionResolveRequest::new(
            file_tree_intent(ContextActionId::Copy, ContextActionTargetKind::MarkdownFile),
            file_tree_readiness(false),
        ),
    )
    .expect("markdown target should resolve");

    assert!(file_resolved.is_none());
    assert_eq!(markdown_resolved.descriptor.id, ContextActionId::Copy);
}
