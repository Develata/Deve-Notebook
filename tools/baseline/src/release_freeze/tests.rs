//! plan_ref:
//!   - 18_release#artifact-identity-and-integrity
//!   - 18_release#release-versioning

use super::workflows::{
    step_block, validate_outer_job_budgets, validate_promotion_assets_step, validate_workflow_texts,
};
use super::{
    ReleaseFreeze, accepted_gap_bindings, android_version_fallback, reject_unconsumed,
    release_notes_root, validate_candidate_contract, validate_registry, verify_candidate_root,
    verify_root,
};
use serde_json::Value;

fn registry_value() -> Value {
    serde_json::from_str(include_str!(
        "../../../../docs/registry/release-freeze.json"
    ))
    .expect("release freeze JSON")
}

fn parse(value: Value) -> ReleaseFreeze {
    serde_json::from_value(value).expect("typed release freeze")
}

#[test]
fn current_release_freeze_matches_workspace_and_workflows() {
    let root = crate::workspace_root::repo_root().expect("workspace root");
    verify_root(&root).expect("current release freeze");
    verify_candidate_root(&root).expect("current candidate freeze");
}

#[test]
fn registry_rejects_unknown_fields() {
    let mut value = registry_value();
    value
        .as_object_mut()
        .expect("object")
        .insert("shadow_authority".to_owned(), Value::Bool(true));

    assert!(serde_json::from_value::<ReleaseFreeze>(value).is_err());
}

#[test]
fn registry_accepts_only_the_user_approved_store_016_gap() {
    let root = crate::workspace_root::repo_root().expect("workspace root");
    let bindings = accepted_gap_bindings(&root).expect("accepted gap bindings");
    assert_eq!(
        bindings.keys().collect::<Vec<_>>(),
        vec![&(
            "case.store-016".to_owned(),
            "gap.watcher.windows-overflow-receipt".to_owned(),
        )]
    );

    let mut value = registry_value();
    value["accepted_gaps"][0]["bindings"][0]["requirement_id"] =
        Value::String("case.store-007".to_owned());
    let registry = parse(value);
    let changelog = include_str!("../../../../CHANGELOG.md");
    let error = super::known_limitations::validate(&registry, changelog)
        .expect_err("STORE-007 must not be accepted");
    assert!(
        error
            .to_string()
            .contains("only the STORE-016 Windows overflow gap")
    );
    let error = reject_unconsumed(&bindings).expect_err("unconsumed accepted gap must fail");
    assert!(
        error
            .to_string()
            .contains("accepted gaps do not match current required tag-ready gaps")
    );
}

#[test]
fn frozen_release_notes_are_derived_from_changelog_and_quantify_thousands() {
    let root = crate::workspace_root::repo_root().expect("workspace root");
    let notes = release_notes_root(&root).expect("frozen release notes");
    assert!(notes.starts_with("## [0.1.0] - 2026-07-26\n"));
    assert!(notes.contains("### Known limitations\n"));
    assert!(notes.contains("数千个外部文件变更"));
    assert!(!notes.contains("大量"));
}

#[test]
fn changelog_rejects_an_unregistered_known_limitation() {
    let registry = parse(registry_value());
    let changelog = include_str!("../../../../CHANGELOG.md");
    let mutated =
        format!("{changelog}\n- **Unregistered limitation**: this is not in release-freeze.json\n");
    let error = super::known_limitations::validate(&registry, &mutated)
        .expect_err("extra known limitation must fail");
    assert!(
        error
            .to_string()
            .contains("must exactly equal the accepted-gap projection")
    );
}

#[test]
fn accepted_gap_public_fields_reject_the_old_unquantified_wording() {
    let mut value = registry_value();
    value["accepted_gaps"][0]["impact"] = Value::String("大量外部变化可能使投影暂时陈旧。".into());
    let registry = parse(value);
    let changelog = include_str!("../../../../CHANGELOG.md");
    let error = super::known_limitations::validate(&registry, changelog)
        .expect_err("old unquantified wording must fail");
    assert!(error.to_string().contains("impact"));
}

#[test]
fn unreleased_content_is_allowed_by_history_check_but_rejected_for_a_candidate() {
    let registry = parse(registry_value());
    let changelog = include_str!("../../../../CHANGELOG.md");
    let mutated = changelog.replace(
        "## [Unreleased]\n",
        "## [Unreleased]\n\n### Added\n- next development change\n",
    );

    super::known_limitations::validate(&registry, &mutated)
        .expect("historical release freeze remains valid");
    let error = super::known_limitations::validate_candidate(&registry, &mutated)
        .expect_err("candidate must keep Unreleased empty");
    assert!(error.to_string().contains("must be empty"));
}

#[test]
fn registry_rejects_unfrozen_macos_alternative() {
    let mut value = registry_value();
    value["artifacts"]["macos_host_dmg"]["one_of"][1] =
        Value::String("artifacts/macos/deve-notebook-{version}-macos-universal.dmg".to_owned());

    let error = validate_registry(&parse(value)).expect_err("universal macOS must fail");
    assert!(error.to_string().contains("exactly x64 and arm64"));
}

#[test]
fn registry_rejects_public_docker_archive() {
    let mut value = registry_value();
    value["artifacts"]["docker_linux_amd64_archive"]["public"] = Value::Bool(true);

    let error = validate_registry(&parse(value)).expect_err("public Docker archive must fail");
    assert!(error.to_string().contains("candidate-internal"));
}

#[test]
fn registry_rejects_windows_absolute_artifact_template() {
    let mut value = registry_value();
    value["artifacts"]["provenance_bundle"]["path"] =
        Value::String("C:/candidate/provenance.bundle".to_owned());

    let error = validate_registry(&parse(value)).expect_err("absolute template must fail");
    assert!(error.to_string().contains("normalized relative"));
}

#[test]
fn registry_rejects_duplicate_case_insensitive_basename() {
    let mut value = registry_value();
    value["artifacts"]["docker_sbom_bundle"]["path"] =
        Value::String("other/PROVENANCE.BUNDLE".to_owned());

    let error = validate_registry(&parse(value)).expect_err("duplicate basename must fail");
    assert!(error.to_string().contains("unique case-insensitive"));
}

#[test]
fn candidate_contract_is_derived_from_assembler_roles() {
    let registry = parse(registry_value());
    validate_candidate_contract(&registry).expect("candidate role contract");
}

#[test]
fn android_fallback_rejects_comment_only_match() {
    let content =
        r#"// versionName = tauriProperties.getProperty("tauri.android.versionName", "0.1.0")"#;
    assert!(android_version_fallback(content).is_err());
}

#[test]
fn promotion_assets_reject_extra_append() {
    let registry = parse(registry_value());
    let promotion = include_str!("../../../../.github/workflows/release.yml");
    let step = step_block(promotion, "Stage unchanged public release assets").expect("asset step");
    let mutated = format!(
        "{step}\n          touch \"$candidate/deve.AppImage\"\n          printf '%s\\n' deve.AppImage >>\"$asset_list\"\n"
    );

    assert!(validate_promotion_assets_step(&mutated, &registry).is_err());
}

#[test]
fn workflow_rejects_commented_macos_allowlist() {
    let registry = parse(registry_value());
    let candidate = include_str!("../../../../.github/workflows/release-candidate.yml");
    let promotion = include_str!("../../../../.github/workflows/release.yml");
    let active = r#""deve-notebook-${VERSION}-macos-x64.dmg"|"deve-notebook-${VERSION}-macos-arm64.dmg") ;;"#;
    let mutated = candidate.replace(
        active,
        &format!("# {active}\n            \"other.dmg\") ;;"),
    );

    assert!(validate_workflow_texts(&mutated, promotion, &registry).is_err());
}

#[test]
fn workflow_rejects_semver_only_latest_classification() {
    let registry = parse(registry_value());
    let candidate = include_str!("../../../../.github/workflows/release-candidate.yml");
    let promotion = include_str!("../../../../.github/workflows/release.yml");
    let mutated = promotion.replace(
        r#"if [[ "$release_channel" == stable ]]; then"#,
        r#"if [[ "${version%%+*}" != *-* ]]; then"#,
    );

    assert!(validate_workflow_texts(candidate, &mutated, &registry).is_err());
}

#[test]
fn workflow_rejects_generated_release_notes_instead_of_the_frozen_changelog() {
    let registry = parse(registry_value());
    let candidate = include_str!("../../../../.github/workflows/release-candidate.yml");
    let promotion = include_str!("../../../../.github/workflows/release.yml");
    let mutated = promotion.replace(
        r#"--notes-file "$release_notes" \"#,
        r#"--generate-notes \"#,
    );

    assert_ne!(mutated, promotion);
    assert!(validate_workflow_texts(candidate, &mutated, &registry).is_err());
}

#[test]
fn workflow_requires_remote_import_and_pvr_receipts_on_the_candidate_head() {
    let registry = parse(registry_value());
    let candidate = include_str!("../../../../.github/workflows/release-candidate.yml");
    let promotion = include_str!("../../../../.github/workflows/release.yml");

    for producer in ["docker.remote-import-browser", "github.pvr-enabled"] {
        let mutated = candidate.replace(
            &format!("--producer {producer}"),
            "--producer omitted.from.candidate",
        );
        assert_ne!(mutated, candidate);
        assert!(validate_workflow_texts(&mutated, promotion, &registry).is_err());
    }
}

#[test]
fn workflow_requires_direct_cargo_audit_version_probe() {
    let registry = parse(registry_value());
    let candidate = include_str!("../../../../.github/workflows/release-candidate.yml");
    let promotion = include_str!("../../../../.github/workflows/release.yml");
    let mutated = candidate.replace(
        r#"run: test "$(cargo-audit --version)" = "cargo-audit 0.22.2""#,
        r#"run: test "$(cargo audit --version)" = "cargo-audit 0.22.2""#,
    );

    assert_ne!(mutated, candidate);
    let error = validate_workflow_texts(&mutated, promotion, &registry)
        .expect_err("Cargo subcommand probe must not satisfy the candidate contract");
    assert!(
        error
            .to_string()
            .contains("candidate direct cargo-audit version verification")
    );
}

#[test]
fn workflow_requires_repository_linkage_and_anonymous_ghcr_pull() {
    let registry = parse(registry_value());
    let candidate = include_str!("../../../../.github/workflows/release-candidate.yml");
    let promotion = include_str!("../../../../.github/workflows/release.yml");
    let candidate_without_source = candidate.replace(
        r#"--label "org.opencontainers.image.source=https://github.com/${GITHUB_REPOSITORY}" \"#,
        r#"--label "org.opencontainers.image.description=missing-source" \"#,
    );
    assert_ne!(candidate_without_source, candidate);
    assert!(validate_workflow_texts(&candidate_without_source, promotion, &registry).is_err());

    let promotion_without_anonymous_pull = promotion.replace(
        r#"DOCKER_CONFIG="$anonymous_config" docker pull "$VERSION_TAG" >/dev/null"#,
        r#"docker pull "$VERSION_TAG" >/dev/null"#,
    );
    assert_ne!(promotion_without_anonymous_pull, promotion);
    assert!(
        validate_workflow_texts(candidate, &promotion_without_anonymous_pull, &registry).is_err()
    );
}

#[test]
fn workflow_outer_timeouts_cover_serial_producer_budgets() {
    let candidate = include_str!("../../../../.github/workflows/release-candidate.yml");
    let native = include_str!("../../../../.github/workflows/release-native.yml");
    validate_outer_job_budgets(candidate, native).expect("current outer budgets");

    let candidate_too_short = candidate.replace(
        "  docker-linux-amd64-smoke:\n    needs: [identity, docker-linux-amd64-build]\n    runs-on: ubuntu-latest\n    timeout-minutes: 240",
        "  docker-linux-amd64-smoke:\n    needs: [identity, docker-linux-amd64-build]\n    runs-on: ubuntu-latest\n    timeout-minutes: 180",
    );
    assert_ne!(candidate_too_short, candidate);
    assert!(validate_outer_job_budgets(&candidate_too_short, native).is_err());
    let native_too_short = native.replace(
        "  desktop-macos-smoke:\n    needs: desktop-macos-build\n    runs-on: macos-latest\n    timeout-minutes: 135",
        "  desktop-macos-smoke:\n    needs: desktop-macos-build\n    runs-on: macos-latest\n    timeout-minutes: 90",
    );
    assert_ne!(native_too_short, native);
    assert!(validate_outer_job_budgets(candidate, &native_too_short).is_err());
}

#[test]
fn workflow_rejects_candidate_mutation_after_preupload_verify() {
    let registry = parse(registry_value());
    let candidate = include_str!("../../../../.github/workflows/release-candidate.yml");
    let promotion = include_str!("../../../../.github/workflows/release.yml");
    let mutated = promotion.replace(
        "            upload=()",
        "            printf evil > \"$candidate/artifacts/windows/deve-notebook-0.1.0-windows-x64.msi\"\n            upload=()",
    );

    assert!(validate_workflow_texts(candidate, &mutated, &registry).is_err());
}

#[test]
fn workflow_rejects_second_release_upload_step() {
    let registry = parse(registry_value());
    let candidate = include_str!("../../../../.github/workflows/release-candidate.yml");
    let promotion = include_str!("../../../../.github/workflows/release.yml");
    let mutated = format!(
        "{promotion}\n      - name: Upload unfrozen Docker archive\n        run: gh release upload \"$GITHUB_REF_NAME\" \"$DEVE_SEALED_ROOT/candidate/artifacts/docker/deve-notebook-0.1.0-linux-amd64.tar\"\n"
    );

    assert!(validate_workflow_texts(candidate, &mutated, &registry).is_err());
}

#[test]
fn workflow_step_extraction_fails_closed() {
    let workflow =
        "steps:\n      - name: One\n        run: true\n      - name: Two\n        run: false\n";
    let block = step_block(workflow, "One").expect("first step");
    assert!(block.contains("run: true"));
    assert!(!block.contains("run: false"));
    assert!(step_block(workflow, "Missing").is_err());
}
