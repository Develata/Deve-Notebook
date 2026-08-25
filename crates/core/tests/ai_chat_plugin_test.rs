// crates/core/tests/ai_chat_plugin_test.rs
//! # AI Chat Plugin 集成测试
//!
//! 验证 ai-chat 资源的加载、默认只读能力和外部插件无 stream host authority 的边界。
//! 不涉及真实 API 调用——仅测试 Rhai 脚本层逻辑。

#[cfg(test)]
mod tests {
    use deve_core::plugin::loader::PluginLoader;
    use std::path::PathBuf;

    fn load_ai_chat() -> Box<dyn deve_core::plugin::runtime::PluginRuntime> {
        let plugin_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("plugins");
        let loader = PluginLoader::new(plugin_dir.clone());
        let plugin = loader
            .load_plugin(&plugin_dir.join("ai-chat"))
            .expect("Failed to prepare ai-chat plugin");
        plugin.activate().expect("activate ai-chat plugin");
        plugin
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
        assert!(caps.allow_net.is_empty());
        assert!(caps.allow_env.is_empty());
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
    fn external_ai_chat_resource_cannot_reach_stream_bridge_without_host_authority() {
        let plugin = load_ai_chat();
        let err = plugin
            .call(
                "chat",
                vec!["test-req-id".into(), "Hello".into(), rhai::Dynamic::UNIT],
            )
            .expect_err("server stream bridge is absent in this plugin-only test");
        assert!(
            err.to_string()
                .contains("Function not found: ai_chat_stream")
        );
    }

    #[test]
    fn external_ai_chat_resource_with_web_arguments_still_has_no_stream_authority() {
        let plugin = load_ai_chat();
        let context = serde_json::json!({
            "current_file": "",
            "current_markdown": "",
            "selection": null,
            "chat_mode": "plan"
        });
        let history = serde_json::json!([]);
        let err = plugin
            .call(
                "chat",
                vec![
                    "test-req-id".into(),
                    "Hello".into(),
                    rhai::serde::to_dynamic(&context).expect("context to dynamic"),
                    rhai::serde::to_dynamic(&history).expect("history to dynamic"),
                ],
            )
            .expect_err("server stream bridge is absent in this plugin-only test");
        assert!(
            err.to_string()
                .contains("Function not found: ai_chat_stream")
        );
    }

    #[test]
    fn external_ai_chat_context_builds_before_stream_authority_denial() {
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

        assert!(detail.contains("Function not found: ai_chat_stream"));
        assert!(
            !detail.contains("SYSTEM_PROMPT"),
            "prompt construction must not fail on hidden constant scope: {detail}"
        );
    }

    #[test]
    fn external_ai_chat_empty_context_still_has_no_stream_authority() {
        let plugin = load_ai_chat();

        let err = plugin
            .call(
                "chat",
                vec!["test-req-id".into(), "Summarize changes".into(), "".into()],
            )
            .expect_err("stream bridge is not configured in this plugin-only test");
        let detail = err.to_string();

        assert!(detail.contains("Function not found: ai_chat_stream"));
    }
}
