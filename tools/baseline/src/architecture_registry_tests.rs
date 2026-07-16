use super::{
    check_operation_catalog_agent_status, coverage_flow_ids, coverage_operation_rows,
    extract_case_refs, extract_registry, metadata_backtick_value, metadata_line_value,
    plan_operation_flow_ids, require_same_flow_ids,
};
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

#[test]
fn plan_and_coverage_flow_ids_ignore_headers_and_match() {
    let plan = "\
| Flow ID (`flow.*`) | Layer |
|---|---|
| `flow.ui.context-action-routing` | II |
| `flow.repo.switch` | UO |";
    let coverage = "\
| Flow ID | Operation File | Acceptance Cases |
|---|---|---|
| `flow.repo.switch` | [`repo_switch.md`](./operations/repo_switch.md) | `REPO-FEAT-01` |
| `flow.ui.context-action-routing` | [`ui_context_action_routing.md`](./operations/ui_context_action_routing.md) | `UI-WEB-007` |";

    let plan_ids = plan_operation_flow_ids(plan).expect("plan ids");
    let coverage_ids = coverage_flow_ids(coverage).expect("coverage ids");

    assert_eq!(plan_ids, coverage_ids);
    require_same_flow_ids(&plan_ids, &coverage_ids).expect("same flow ids");
}

#[test]
fn plan_and_coverage_flow_ids_report_missing_projection() {
    let plan = "\
| Flow ID (`flow.*`) | Layer |
|---|---|
| `flow.ui.context-action-routing` | II |
| `flow.repo.switch` | UO |";
    let coverage = "\
| Flow ID | Operation File | Acceptance Cases |
|---|---|---|
| `flow.repo.switch` | [`repo_switch.md`](./operations/repo_switch.md) | `REPO-FEAT-01` |";

    let err = require_same_flow_ids(
        &plan_operation_flow_ids(plan).expect("plan ids"),
        &coverage_flow_ids(coverage).expect("coverage ids"),
    )
    .expect_err("missing coverage should fail")
    .to_string();

    assert!(err.contains("missing in coverage: flow.ui.context-action-routing"));
}

#[test]
fn operation_catalog_agent_status_requires_rust_baseline_binding() {
    let agents = "\
| `20_operations_catalog#opid-catalog` | `## 1. Scope & Authority` | operation-flow 目录唯一权威（OpId catalog）；由 deve_baseline architecture-registry 绑定 |
| `20_operations_catalog#extension-point-index` | `## 4. Extension Point Index` | 暴露给 plugins/host 的扩展点索引；由 deve_baseline architecture-registry 绑定 |
| `20_operations_catalog#replacement-point-index` | `## 5. Replacement Point Index` | feature-flag 可替换点索引；由 deve_baseline architecture-registry 绑定 |
| `20_operations_catalog#configuration-entry-index` | `## 6. Configuration Entry Index` | 配置入口主索引（定义 defer 各原章）；由 deve_baseline architecture-registry 绑定 |";

    check_operation_catalog_agent_status(agents).expect("valid status");

    let stale = agents.replace(
        "由 deve_baseline architecture-registry 绑定",
        "planned/no-code-yet",
    );
    let err = check_operation_catalog_agent_status(&stale)
        .expect_err("planned status should fail")
        .to_string();
    assert!(err.contains("operation catalog registry status"));
}

#[test]
fn coverage_operation_rows_bind_flow_to_file() {
    let coverage = "\
| Flow ID | Operation File | Acceptance Cases |
|---|---|---|
| `flow.ui.context-action-routing` | [`ui_context_action_routing.md`](./operations/ui_context_action_routing.md) | `UI-WEB-007` |";

    let rows = coverage_operation_rows(coverage).expect("coverage rows");

    assert_eq!(
        rows.get("flow.ui.context-action-routing")
            .map(String::as_str),
        Some("operations/ui_context_action_routing.md")
    );
}

#[test]
fn flow_id_parser_rejects_trailing_or_empty_segments() {
    let plan = "\
| Flow ID (`flow.*`) | Layer |
|---|---|
| `flow.a.` | II |
| `flow.a..b` | II |
| `flow.valid-name` | II |";

    let ids = plan_operation_flow_ids(plan).expect("flow ids");

    assert_eq!(ids, BTreeSet::from(["flow.valid-name".to_string()]));
}
