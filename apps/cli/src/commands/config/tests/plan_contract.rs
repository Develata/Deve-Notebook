use super::super::schema::{config_key_specs, supported_config_keys};
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn supported_config_keys_match_settings_plan_tables() {
    let docs = include_str!("../../../../../../docs/plan/15_settings.md");
    let documented = extract_documented_config_rows(docs)
        .into_keys()
        .collect::<BTreeSet<_>>();
    let supported = supported_config_keys().collect::<BTreeSet<_>>();

    assert_eq!(documented, supported);
}

#[test]
fn supported_config_schema_matches_settings_plan_type_and_choices() {
    let docs = include_str!("../../../../../../docs/plan/15_settings.md");
    let documented = extract_documented_config_rows(docs);

    for spec in config_key_specs() {
        let row = documented
            .get(spec.key)
            .unwrap_or_else(|| panic!("missing documented config key {}", spec.key));
        assert_eq!(
            row.value_type, spec.plan_type,
            "type mismatch for {}",
            spec.key
        );
        if !spec.choices.is_empty() || !row.choices.is_empty() {
            let expected = spec
                .choices
                .iter()
                .map(|choice| choice.to_string())
                .collect::<BTreeSet<_>>();
            assert_eq!(row.choices, expected, "choices mismatch for {}", spec.key);
        }
    }
}

struct DocumentedConfigRow {
    value_type: String,
    choices: BTreeSet<String>,
}

fn extract_documented_config_rows(docs: &str) -> BTreeMap<&str, DocumentedConfigRow> {
    let mut rows = BTreeMap::new();
    let mut in_config_section = false;
    for line in docs.lines() {
        if line.starts_with("## 3.") {
            break;
        }
        if line.starts_with("### 2.") {
            in_config_section = true;
            continue;
        }
        if !in_config_section || !line.starts_with('|') {
            continue;
        }
        let cells = line.split('|').map(str::trim).collect::<Vec<_>>();
        let Some(key_cell) = cells.get(1).copied() else {
            continue;
        };
        if !key_cell.starts_with('`') {
            continue;
        }
        let Some(value_type) = cells.get(2).copied() else {
            continue;
        };
        let description = if cells.len() > 5 {
            cells[4..cells.len() - 1].join("|")
        } else {
            cells.get(4).copied().unwrap_or_default().to_string()
        };
        for key in key_cell.split("<br>").flat_map(|cell| cell.split('/')) {
            let key = key.trim().trim_matches('`');
            if key.starts_with("DEVE_") || key.is_empty() || key.contains("[]") {
                continue;
            }
            rows.insert(
                key,
                DocumentedConfigRow {
                    value_type: value_type.to_string(),
                    choices: extract_choice_tokens(&description),
                },
            );
        }
    }
    rows
}

fn extract_choice_tokens(description: &str) -> BTreeSet<String> {
    description
        .split('`')
        .enumerate()
        .filter_map(|(index, token)| (index % 2 == 1).then_some(token.trim()))
        .filter(|token| !token.is_empty())
        .filter(|token| {
            !token.starts_with("DEVE_")
                && !token.contains(".md")
                && !token.contains('§')
                && !token.contains(' ')
        })
        .map(str::to_string)
        .collect()
}
