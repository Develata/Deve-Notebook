// crates/core/tests/ai_chat_plugin_test.rs
//! # AI Chat Plugin 集成测试
//!
//! 验证内置 ai-chat 插件的加载、默认只读能力和配置构建。
//! 不涉及真实 API 调用——仅测试 Rhai 脚本层逻辑。

#[cfg(test)]
mod tests {
    use deve_core::plugin::loader::PluginLoader;
    use std::path::PathBuf;
    use std::sync::Mutex;

    static AI_ENV_LOCK: Mutex<()> = Mutex::new(());

    struct EnvGuard {
        previous: Vec<(&'static str, Option<String>)>,
    }

    impl EnvGuard {
        fn set(values: &[(&'static str, Option<&'static str>)]) -> Self {
            let previous = values
                .iter()
                .map(|(key, _)| (*key, std::env::var(key).ok()))
                .collect();
            for (key, value) in values {
                // SAFETY: AI_ENV_LOCK serializes these tests, and all keys are
                // process-level plugin configuration variables.
                unsafe {
                    match value {
                        Some(value) => std::env::set_var(key, value),
                        None => std::env::remove_var(key),
                    }
                }
            }
            Self { previous }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for (key, value) in self.previous.drain(..).rev() {
                // SAFETY: AI_ENV_LOCK is still held by the test while this
                // guard restores process-level plugin configuration variables.
                unsafe {
                    match value {
                        Some(value) => std::env::set_var(key, value),
                        None => std::env::remove_var(key),
                    }
                }
            }
        }
    }

    fn load_ai_chat() -> Box<dyn deve_core::plugin::runtime::PluginRuntime> {
        let plugin_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("plugins");
        let loader = PluginLoader::new(plugin_dir.clone());
        loader
            .load_plugin(&plugin_dir.join("ai-chat"))
            .expect("Failed to load ai-chat plugin")
    }

    #[test]
    fn test_ai_chat_plugin_loads() {
        let plugin = load_ai_chat();
        assert_eq!(plugin.manifest().id, "ai-chat");
        assert_eq!(plugin.manifest().name, "AI Chat");
        assert_eq!(plugin.manifest().version, "0.1.0");
        assert_eq!(plugin.manifest().entry, "main.rhai");
    }

    #[test]
    fn test_ai_chat_manifest_capabilities() {
        let plugin = load_ai_chat();
        let caps = &plugin.manifest().capabilities;
        assert!(caps.allow_net.contains(&"api.openai.com".to_string()));
        assert!(caps.allow_env.contains(&"AI_API_KEY".to_string()));
        assert!(caps.allow_fs_read.is_empty());
        assert!(caps.allow_fs_write.is_empty());
        assert!(!caps.allow_source_control);
        assert!(!caps.allow_search);
        assert!(!caps.allow_project_tree);
        assert!(!caps.allow_skill);
    }

    #[test]
    fn test_run_tool_read_file_is_disabled_by_default() {
        let plugin = load_ai_chat();
        let result = plugin
            .call(
                "run_tool",
                vec!["read_file".into(), r#"{"path":"manifest.json"}"#.into()],
            )
            .expect("run_tool should return disabled error");

        let content = result.into_string().unwrap();
        assert!(
            content.contains("disabled by default"),
            "Should deny default file tool, got: {content}"
        );
    }

    #[test]
    fn test_run_tool_source_control_write_is_disabled_by_default() {
        let plugin = load_ai_chat();
        let result = plugin
            .call(
                "run_tool",
                vec!["git_commit".into(), r#"{"message":"test"}"#.into()],
            )
            .expect("run_tool should return disabled error");

        let content = result.into_string().unwrap();
        assert!(
            content.contains("disabled by default"),
            "Should deny default source-control tool, got: {content}"
        );
    }

    #[test]
    fn test_internal_config_defaults() {
        let _guard = AI_ENV_LOCK.lock().expect("ai env lock");
        let _env = EnvGuard::set(&[
            ("AI_BASE_URL", None),
            ("AI_API_KEY", None),
            ("AI_MODEL", None),
            ("OPENAI_API_KEY", None),
            ("ANTHROPIC_API_KEY", None),
        ]);
        let plugin = load_ai_chat();
        let result = plugin
            .call("_build_config", vec![])
            .expect("_build_config should work internally");

        // Result is a Rhai Map
        let config: rhai::Map = rhai::serde::from_dynamic(&result).unwrap();
        let model = config
            .get("model")
            .and_then(|v| v.clone().into_string().ok())
            .unwrap_or_default();
        // 默认 model 为 gpt-4o-mini（当环境变量未设置时）
        assert_eq!(model, "gpt-4o-mini");
    }

    #[test]
    fn test_system_prompt_includes_current_markdown_and_mode_boundary() {
        let plugin = load_ai_chat();
        let context = serde_json::json!({
            "current_file": "notes/today.md",
            "current_markdown": "# Today\n\n- ship Native AI Chat minimum",
            "chat_mode": "build",
            "selection": {"text": "ship Native AI Chat minimum"}
        });
        let context = rhai::serde::to_dynamic(&context).expect("context to dynamic");

        let result = plugin
            .call("build_system_prompt", vec![context])
            .expect("build prompt");
        let prompt = result.into_string().expect("prompt string");

        assert!(prompt.contains("Current file: notes/today.md"));
        assert!(prompt.contains("BUILD mode"));
        assert!(prompt.contains("Current markdown content"));
        assert!(prompt.contains("ship Native AI Chat minimum"));
        assert!(prompt.contains("do not execute workspace"));
        assert!(prompt.contains("Selected text"));
    }

    #[test]
    fn test_chat_without_api_key_returns_error() {
        let _guard = AI_ENV_LOCK.lock().expect("ai env lock");
        let _env = EnvGuard::set(&[
            ("AI_API_KEY", None),
            ("OPENAI_API_KEY", None),
            ("ANTHROPIC_API_KEY", None),
        ]);
        let plugin = load_ai_chat();
        // 调用 chat 时没有 API key 应返回错误消息 (不 panic)
        let result = plugin
            .call(
                "chat",
                vec!["test-req-id".into(), "Hello".into(), rhai::Dynamic::UNIT],
            )
            .expect("chat should not panic");

        // 应返回结构化 error 结果，server 会把它转换为 PluginResponse.error。
        let response: rhai::Map = rhai::serde::from_dynamic(&result).unwrap();
        let kind = response
            .get("type")
            .and_then(|v| v.clone().into_string().ok())
            .unwrap_or_default();
        let content = response
            .get("content")
            .and_then(|v| v.clone().into_string().ok())
            .unwrap_or_default();
        assert_eq!(kind, "error");
        assert!(
            content.contains("API key") || content.contains("Error"),
            "Should return API key error, got: {}",
            content
        );
    }

    #[test]
    fn test_chat_without_api_key_returns_error_with_web_arguments() {
        let _guard = AI_ENV_LOCK.lock().expect("ai env lock");
        let _env = EnvGuard::set(&[
            ("AI_API_KEY", None),
            ("OPENAI_API_KEY", None),
            ("ANTHROPIC_API_KEY", None),
        ]);
        let plugin = load_ai_chat();
        let context = serde_json::json!({
            "current_file": "",
            "current_markdown": "",
            "selection": null,
            "chat_mode": "plan"
        });
        let history = serde_json::json!([]);
        let result = plugin
            .call(
                "chat",
                vec![
                    "test-req-id".into(),
                    "Hello".into(),
                    rhai::serde::to_dynamic(&context).expect("context to dynamic"),
                    rhai::serde::to_dynamic(&history).expect("history to dynamic"),
                ],
            )
            .expect("chat should not panic with web arguments");

        let response: rhai::Map = rhai::serde::from_dynamic(&result).unwrap();
        let kind = response
            .get("type")
            .and_then(|v| v.clone().into_string().ok())
            .unwrap_or_default();
        let content = response
            .get("content")
            .and_then(|v| v.clone().into_string().ok())
            .unwrap_or_default();
        assert_eq!(kind, "error");
        assert!(content.contains("API key"), "got: {content}");
    }

    #[test]
    fn test_chat_with_api_key_reaches_stream_bridge() {
        let _guard = AI_ENV_LOCK.lock().expect("ai env lock");
        let _env = EnvGuard::set(&[
            ("AI_API_KEY", Some("deve-test-key")),
            ("AI_BASE_URL", Some("http://127.0.0.1:1/v1")),
            ("AI_MODEL", Some("deve-test-model")),
            ("OPENAI_API_KEY", None),
            ("ANTHROPIC_API_KEY", None),
        ]);
        let plugin = load_ai_chat();
        let context = serde_json::json!({
            "current_file": "notes/current.md",
            "current_markdown": "# Current\n\nNative AI positive path.",
            "chat_mode": "plan"
        });
        let context = rhai::serde::to_dynamic(&context).expect("context to dynamic");

        let err = plugin
            .call(
                "chat",
                vec!["test-req-id".into(), "Summarize".into(), context],
            )
            .expect_err("stream bridge is not configured in this plugin-only test");
        let detail = err.to_string();

        assert!(
            detail.contains("Chat stream handler not configured"),
            "chat should reach the stream bridge, got: {detail}"
        );
        assert!(
            !detail.contains("SYSTEM_PROMPT"),
            "prompt construction must not fail on hidden constant scope: {detail}"
        );
    }

    #[test]
    fn test_chat_with_api_key_accepts_empty_string_context_as_no_context() {
        let _guard = AI_ENV_LOCK.lock().expect("ai env lock");
        let _env = EnvGuard::set(&[
            ("AI_API_KEY", Some("deve-test-key")),
            ("AI_BASE_URL", Some("http://127.0.0.1:1/v1")),
            ("AI_MODEL", Some("deve-test-model")),
            ("OPENAI_API_KEY", None),
            ("ANTHROPIC_API_KEY", None),
        ]);
        let plugin = load_ai_chat();

        let err = plugin
            .call(
                "chat",
                vec!["test-req-id".into(), "Summarize changes".into(), "".into()],
            )
            .expect_err("stream bridge is not configured in this plugin-only test");
        let detail = err.to_string();

        assert!(
            detail.contains("Chat stream handler not configured"),
            "empty context should be treated as no context and reach the stream bridge, got: {detail}"
        );
    }
}
