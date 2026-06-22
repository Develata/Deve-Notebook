use super::{extract_case_refs, extract_registry, metadata_backtick_value, metadata_line_value};
use std::collections::BTreeSet;

#[test]
fn extracts_registry_between_exact_markers() {
    let content = "\
before
<!-- flow-registry:start -->
- `login`
- `repo-switch`
<!-- flow-registry:end -->
after";
    let items = extract_registry(
        content,
        "<!-- flow-registry:start -->",
        "<!-- flow-registry:end -->",
    )
    .expect("registry");

    assert_eq!(items, ["login", "repo-switch"]);
}

#[test]
fn metadata_helpers_match_operation_files() {
    let content = "\
- `Flow ID`: `flow.ai.chat`
- `Related Acceptance Cases`: `AI-FEAT-01`, `AI-002`";

    assert_eq!(
        metadata_backtick_value(content, "Flow ID").as_deref(),
        Some("flow.ai.chat")
    );
    assert_eq!(
        metadata_line_value(content, "`Related Acceptance Cases`").as_deref(),
        Some("`AI-FEAT-01`, `AI-002`")
    );
}

#[test]
fn case_refs_follow_shell_regex_shape() {
    let refs = extract_case_refs("`CMD-004A`, `AI-FEAT-01`, `REL-003`");

    assert_eq!(
        refs,
        BTreeSet::from([
            "AI-FEAT-01".to_string(),
            "CMD-004".to_string(),
            "REL-003".to_string(),
        ])
    );
}
