//! plan_ref:
//!   - 18_release#artifact-identity-and-integrity
//!   - 18_release#release-versioning

use super::workflows::{step_block, validate_promotion_assets_step, validate_workflow_texts};
use super::{
    ReleaseFreeze, android_version_fallback, validate_candidate_contract, validate_registry,
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
