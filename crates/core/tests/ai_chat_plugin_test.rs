// crates/core/tests/ai_chat_plugin_test.rs
//! # AI Chat Plugin 集成测试
//!
//! 验证内置 ai-chat 插件的加载、默认只读能力和配置构建。
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
        assert!(!caps.allow_mcp);
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
        // 清除可能存在的环境变量 (不影响其他测试——读取时不会修改)
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
    fn test_chat_without_api_key_returns_error() {
        let plugin = load_ai_chat();
        // 调用 chat 时没有 API key 应返回错误消息 (不 panic)
        let result = plugin
            .call(
                "chat",
                vec!["test-req-id".into(), "Hello".into(), rhai::Dynamic::UNIT],
            )
            .expect("chat should not panic");

        // 应返回 map 包含 error 信息
        let response: rhai::Map = rhai::serde::from_dynamic(&result).unwrap();
        let content = response
            .get("content")
            .and_then(|v| v.clone().into_string().ok())
            .unwrap_or_default();
        assert!(
            content.contains("API key") || content.contains("Error"),
            "Should return API key error, got: {}",
            content
        );
    }
}
