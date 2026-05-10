use super::schema::{config_key_specs, supported_config_keys};
use super::set_in_file;
use deve_core::config::{AppProfile, Config};
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn set_core_key_writes_runtime_config() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("config.toml");

    set_in_file(&path, "profile", "low-spec").expect("set profile");

    let output = std::fs::read_to_string(path).expect("read config");
    let config: Config = toml::from_str(&output).expect("valid config");
    assert_eq!(config.profile, AppProfile::LowSpec);
}

#[test]
fn set_ui_key_is_preserved_without_breaking_runtime_config() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("config.toml");

    set_in_file(&path, "ui.sidebar_width", "300").expect("set ui");

    let output = std::fs::read_to_string(path).expect("read config");
    let config = toml::from_str::<Config>(&output).expect("runtime-compatible config");
    assert_eq!(config.ui.sidebar_width, 300);
    assert!(output.contains("sidebar_width = 300"));
}

#[test]
fn set_rejects_unknown_key() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("config.toml");
    let original = "profile = \"standard\"\n";
    std::fs::write(&path, original).expect("seed config");

    let err = set_in_file(&path, "unknown.key", "1").expect_err("reject key");
    assert!(err.to_string().contains("Unsupported config key"));

    let err = set_in_file(&path, "server.settings.api_enabled", "true").expect_err("reject future");
    assert!(err.to_string().contains("Unsupported config key"));
    assert_eq!(
        std::fs::read_to_string(&path).expect("read config"),
        original
    );
}

#[test]
fn set_rejects_invalid_value_without_rewriting_config() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("config.toml");
    let original = "profile = \"standard\"\n";
    std::fs::write(&path, original).expect("seed config");

    let invalid_choice = set_in_file(&path, "profile", "invalid").expect_err("reject choice");
    assert!(invalid_choice.to_string().contains("Invalid value"));
    assert_eq!(
        std::fs::read_to_string(&path).expect("read config"),
        original
    );

    let invalid_integer = set_in_file(&path, "ui.sidebar_width", "-1").expect_err("reject integer");
    assert!(
        invalid_integer
            .to_string()
            .contains("Integer config values must be non-negative")
    );
    assert_eq!(
        std::fs::read_to_string(&path).expect("read config"),
        original
    );
}

#[test]
fn set_rejects_nested_key_when_parent_is_scalar() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("config.toml");
    let original = "ui = \"scalar\"\n";
    std::fs::write(&path, original).expect("seed config");

    let err = set_in_file(&path, "ui.sidebar_width", "300").expect_err("reject scalar parent");
    assert!(err.to_string().contains("ui is already a scalar"));
    assert_eq!(
        std::fs::read_to_string(&path).expect("read config"),
        original
    );
}

#[test]
fn supported_config_keys_match_settings_plan_tables() {
    let docs = include_str!("../../../../../docs/plan/13_settings.md");
    let documented = extract_documented_config_rows(docs)
        .into_keys()
        .collect::<BTreeSet<_>>();
    let supported = supported_config_keys().collect::<BTreeSet<_>>();

    assert_eq!(documented, supported);
}

#[test]
fn supported_config_schema_matches_settings_plan_type_and_choices() {
    let docs = include_str!("../../../../../docs/plan/13_settings.md");
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
            if key.starts_with("DEVE_") || key.is_empty() {
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
