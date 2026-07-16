//! plan_ref:
//!   - 16_ai_agent#native-ai-chat-runtime
//!
/// 插件响应类型别名。
pub type PluginResponse = Option<(
    String,
    Option<serde_json::Value>,
    Option<deve_core::protocol::ServerError>,
)>;
