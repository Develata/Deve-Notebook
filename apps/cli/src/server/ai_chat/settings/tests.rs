//! plan_ref:
//!   - 15_settings#native-ai-provider-settings

use super::source::{default_snapshot, validate_snapshot};
use super::*;
use std::collections::BTreeMap;

fn values(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
    entries
        .iter()
        .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
        .collect()
}

#[test]
fn canonical_environment_is_whole_group_authority() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("ai.env"),
        "AI_API_KEY=stored-secret\nAI_MODEL=stored-model\n",
    )
    .unwrap();
    let runtime = NativeAiProviderSettingsRuntime::from_sources(
        dir.path(),
        values(&[("AI_MODEL", "environment-model")]),
    )
    .unwrap();
    let snapshot = runtime.snapshot().unwrap();
    let projection = runtime.projection().unwrap();

    assert_eq!(snapshot.model, "environment-model");
    assert!(snapshot.api_key.is_empty());
    assert_eq!(projection.source, SettingsSource::Environment);
    assert!(!projection.writable);
}

#[test]
fn provider_key_aliases_select_exact_provider_authority() {
    let dir = tempfile::tempdir().unwrap();
    let openai = NativeAiProviderSettingsRuntime::from_sources(
        dir.path(),
        values(&[("OPENAI_API_KEY", "openai-key")]),
    )
    .unwrap();
    let openai = openai.snapshot().unwrap();
    assert_eq!(openai.provider, ProviderProtocol::OpenaiChatCompletions);
    assert_eq!(openai.base_url, source::DEFAULT_BASE_URL);
    assert_eq!(openai.api_key, "openai-key");

    let anthropic = NativeAiProviderSettingsRuntime::from_sources(
        dir.path(),
        values(&[("ANTHROPIC_API_KEY", "anthropic-key")]),
    )
    .unwrap();
    let anthropic = anthropic.snapshot().unwrap();
    assert_eq!(anthropic.provider, ProviderProtocol::AnthropicMessages);
    assert_eq!(anthropic.base_url, "https://api.anthropic.com/v1");
    assert_eq!(anthropic.model, "claude-sonnet-4-6");
    assert_eq!(anthropic.api_key, "anthropic-key");
}

#[test]
fn ambiguous_provider_key_aliases_fail_closed() {
    let dir = tempfile::tempdir().unwrap();
    let error = NativeAiProviderSettingsRuntime::from_sources(
        dir.path(),
        values(&[
            ("OPENAI_API_KEY", "openai-key"),
            ("ANTHROPIC_API_KEY", "anthropic-key"),
        ]),
    )
    .err()
    .expect("dual aliases must be rejected");
    assert!(
        error
            .to_string()
            .contains("ambiguous AI provider key aliases")
    );
}

#[test]
fn ui_replace_is_atomic_redacted_and_revision_bound() {
    let dir = tempfile::tempdir().unwrap();
    let runtime =
        NativeAiProviderSettingsRuntime::from_sources(dir.path(), BTreeMap::new()).unwrap();
    let saved = runtime
        .replace(ReplaceProviderSettings {
            expected_revision: 1,
            provider: ProviderProtocol::OpenaiResponses,
            base_url: "https://api.openai.com/v1".into(),
            model: "model-a".into(),
            max_tokens: 2048,
            api_key: Some("fixture-secret".into()),
            clear_api_key: false,
        })
        .unwrap();

    assert_eq!(saved.revision, 2);
    assert!(saved.key_configured);
    let encoded = serde_json::to_string(&saved).unwrap();
    assert!(!encoded.contains("fixture-secret"));
    let persisted = std::fs::read_to_string(dir.path().join("ai.env")).unwrap();
    assert!(persisted.contains("AI_API_KEY=\"fixture-secret\""));
    assert_eq!(
        runtime
            .replace(ReplaceProviderSettings {
                expected_revision: 1,
                provider: ProviderProtocol::OpenaiResponses,
                base_url: "https://api.openai.com/v1".into(),
                model: "model-b".into(),
                max_tokens: 2048,
                api_key: None,
                clear_api_key: false,
            })
            .unwrap_err(),
        ReplaceError::RevisionConflict
    );
    assert_eq!(runtime.snapshot().unwrap().model, "model-a");
}

#[test]
fn unknown_ai_env_key_fails_closed() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("ai.env"), "AI_HEADERS=unsafe\n").unwrap();
    assert!(NativeAiProviderSettingsRuntime::from_sources(dir.path(), BTreeMap::new()).is_err());
}

#[test]
fn duplicate_ai_env_key_fails_closed() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("ai.env"),
        "AI_MODEL=first\nAI_MODEL=second\n",
    )
    .unwrap();
    assert!(NativeAiProviderSettingsRuntime::from_sources(dir.path(), BTreeMap::new()).is_err());
}

#[test]
fn provider_url_requires_https_except_loopback() {
    let mut snapshot = default_snapshot();
    snapshot.base_url = "http://provider.example/v1".into();
    assert!(validate_snapshot(&snapshot).is_err());
    snapshot.base_url = "http://127.0.0.1:11434/v1".into();
    assert!(validate_snapshot(&snapshot).is_ok());
}

#[test]
fn ui_store_never_rewrites_project_root_env() {
    let project = tempfile::tempdir().unwrap();
    let data = project.path().join("data");
    std::fs::create_dir(&data).unwrap();
    let root_env = project.path().join(".env");
    std::fs::write(&root_env, "AUTH_USER=operator\n").unwrap();
    let runtime = NativeAiProviderSettingsRuntime::from_sources(&data, BTreeMap::new()).unwrap();

    runtime
        .replace(ReplaceProviderSettings {
            expected_revision: 1,
            provider: ProviderProtocol::OpenaiChatCompletions,
            base_url: source::DEFAULT_BASE_URL.into(),
            model: source::DEFAULT_MODEL.into(),
            max_tokens: source::DEFAULT_MAX_TOKENS,
            api_key: Some("key-$-quoted".into()),
            clear_api_key: false,
        })
        .unwrap();

    assert_eq!(
        std::fs::read_to_string(root_env).unwrap(),
        "AUTH_USER=operator\n"
    );
    let reloaded = NativeAiProviderSettingsRuntime::from_sources(&data, BTreeMap::new()).unwrap();
    assert_eq!(reloaded.snapshot().unwrap().api_key, "key-$-quoted");
}

#[test]
fn ai_env_store_rejects_app_data_parent_replacement() {
    let container = tempfile::tempdir().unwrap();
    let app_data = container.path().join("app-data");
    let files = app_data.join("files");
    std::fs::create_dir_all(&files).unwrap();
    let runtime = NativeAiProviderSettingsRuntime::from_sources(&files, BTreeMap::new()).unwrap();

    let retired = container.path().join("retired-app-data");
    std::fs::rename(&app_data, &retired).unwrap();
    std::fs::create_dir_all(&files).unwrap();

    let failure = runtime
        .replace(ReplaceProviderSettings {
            expected_revision: 1,
            provider: ProviderProtocol::OpenaiChatCompletions,
            base_url: source::DEFAULT_BASE_URL.into(),
            model: source::DEFAULT_MODEL.into(),
            max_tokens: source::DEFAULT_MAX_TOKENS,
            api_key: Some("never-published".into()),
            clear_api_key: false,
        })
        .expect_err("captured files/app-data lineage must reject replacement");

    assert_eq!(failure, ReplaceError::Persistence);
    assert!(!files.join("ai.env").exists());
    assert!(runtime.snapshot().unwrap().api_key.is_empty());
}

#[test]
fn post_replace_durability_failure_seals_runtime_until_reload() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join(".ai-env-fail-after-replace");
    std::fs::write(&marker, "fail").unwrap();
    let runtime =
        NativeAiProviderSettingsRuntime::from_sources(dir.path(), BTreeMap::new()).unwrap();
    let failure = runtime
        .replace(ReplaceProviderSettings {
            expected_revision: 1,
            provider: ProviderProtocol::OpenaiResponses,
            base_url: source::DEFAULT_BASE_URL.into(),
            model: "published-model".into(),
            max_tokens: source::DEFAULT_MAX_TOKENS,
            api_key: Some("published-key".into()),
            clear_api_key: false,
        })
        .expect_err("post-replace failure must be reported");
    assert_eq!(failure, ReplaceError::Persistence);
    assert!(runtime.snapshot().is_err());
    assert!(runtime.projection().is_err());

    std::fs::remove_file(marker).unwrap();
    let reloaded =
        NativeAiProviderSettingsRuntime::from_sources(dir.path(), BTreeMap::new()).unwrap();
    assert_eq!(reloaded.snapshot().unwrap().model, "published-model");
    assert_eq!(reloaded.snapshot().unwrap().api_key, "published-key");
}
