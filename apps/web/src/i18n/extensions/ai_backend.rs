//! plan_ref:
//!   - 13_i18n#i18n-keys-reference
//!   - 16_ai_agent#trusted-agent-bridge

use crate::i18n::Locale;

use super::trusted_cli_unavailable;

pub fn ai_backend_reason(locale: Locale, reason: &str) -> String {
    let reason = reason.trim();
    if let Some(detail) = reason.strip_prefix("AI backend capability probe failed: ") {
        return match locale {
            Locale::En => reason.to_string(),
            Locale::Zh => format!("AI 后端能力探测失败：{detail}"),
        };
    }
    if let Some(backend) = reason.strip_prefix("effective backend is ") {
        let backend = ai_backend_label(locale, backend);
        return match locale {
            Locale::En => format!("Effective backend is {backend}"),
            Locale::Zh => format!("当前有效后端为{backend}"),
        };
    }
    match (locale, reason) {
        (_, "") => trusted_cli_unavailable(locale).to_string(),
        (Locale::En, "external agent disabled") => "External agent disabled".to_string(),
        (Locale::Zh, "external agent disabled") => "外部 Agent 已禁用".to_string(),
        (Locale::En, "trusted mode required") => "Trusted mode required".to_string(),
        (Locale::Zh, "trusted mode required") => "需要启用受信任模式".to_string(),
        (Locale::En, "trusted-cli unavailable") => "Trusted CLI unavailable".to_string(),
        (Locale::Zh, "trusted-cli unavailable") => "受信任 CLI 不可用".to_string(),
        (Locale::En, "native AI disabled by config") => "Native AI disabled by config".to_string(),
        (Locale::Zh, "native AI disabled by config") => "原生 AI 已被配置禁用".to_string(),
        (Locale::En, "trusted-cli explicitly requested") => {
            "Trusted CLI explicitly requested".to_string()
        }
        (Locale::Zh, "trusted-cli explicitly requested") => "已显式请求受信任 CLI".to_string(),
        (Locale::En, "no AI backend available") => "No AI backend available".to_string(),
        (Locale::Zh, "no AI backend available") => "没有可用的 AI 后端".to_string(),
        (Locale::En, "AI backend capability response is invalid") => {
            "AI backend capability response is invalid".to_string()
        }
        (Locale::Zh, "AI backend capability response is invalid") => {
            "AI 后端能力响应无效".to_string()
        }
        (Locale::En, "AI backend capability probe failed") => {
            "AI backend capability probe failed".to_string()
        }
        (Locale::Zh, "AI backend capability probe failed") => "AI 后端能力探测失败".to_string(),
        (Locale::En, "AI backend policy unavailable") => {
            "AI backend policy unavailable".to_string()
        }
        (Locale::Zh, "AI backend policy unavailable") => "AI 后端策略不可用".to_string(),
        (Locale::En, "AGENT_CLI_PATH required") => "AGENT_CLI_PATH required".to_string(),
        (Locale::Zh, "AGENT_CLI_PATH required") => "需要设置 AGENT_CLI_PATH".to_string(),
        (Locale::En, "AGENT_CLI_PATH must be absolute") => {
            "AGENT_CLI_PATH must be absolute".to_string()
        }
        (Locale::Zh, "AGENT_CLI_PATH must be absolute") => {
            "AGENT_CLI_PATH 必须是绝对路径".to_string()
        }
        (Locale::En, "AGENT_CLI_PATH must point to an executable file") => {
            "AGENT_CLI_PATH must point to an executable file".to_string()
        }
        (Locale::Zh, "AGENT_CLI_PATH must point to an executable file") => {
            "AGENT_CLI_PATH 必须指向可执行文件".to_string()
        }
        _ => reason.to_string(),
    }
}

fn ai_backend_label(locale: Locale, backend: &str) -> String {
    match (locale, backend) {
        (Locale::En, "native") => "Native AI".to_string(),
        (Locale::Zh, "native") => "原生 AI".to_string(),
        (Locale::En, "trusted-cli") => "Trusted CLI".to_string(),
        (Locale::Zh, "trusted-cli") => "受信任 CLI".to_string(),
        _ => backend.to_string(),
    }
}

pub fn ai_backend_fallback(locale: Locale, reason: &str) -> String {
    let reason = ai_backend_reason(locale, reason);
    match locale {
        Locale::En => format!("AI backend changed by runtime policy. Reason: {reason}"),
        Locale::Zh => format!("AI 后端已按运行时策略切换。原因：{reason}"),
    }
}

#[cfg(test)]
mod tests {
    use super::{ai_backend_fallback, ai_backend_reason};
    use crate::i18n::Locale;

    #[test]
    fn ai_backend_reasons_are_localized_for_ui_surfaces() {
        assert_eq!(
            ai_backend_reason(Locale::En, "external agent disabled"),
            "External agent disabled"
        );
        assert_eq!(
            ai_backend_reason(Locale::Zh, "external agent disabled"),
            "外部 Agent 已禁用"
        );
        assert_eq!(
            ai_backend_reason(Locale::Zh, "native AI disabled by config"),
            "原生 AI 已被配置禁用"
        );
        assert_eq!(
            ai_backend_reason(Locale::Zh, "trusted-cli unavailable"),
            "受信任 CLI 不可用"
        );
        assert_eq!(
            ai_backend_reason(Locale::Zh, "AGENT_CLI_PATH required"),
            "需要设置 AGENT_CLI_PATH"
        );
        assert_eq!(
            ai_backend_reason(Locale::Zh, "AI backend policy unavailable"),
            "AI 后端策略不可用"
        );
        assert_eq!(
            ai_backend_reason(Locale::Zh, "effective backend is trusted-cli"),
            "当前有效后端为受信任 CLI"
        );
        assert_eq!(
            ai_backend_reason(Locale::Zh, "AI backend capability probe failed: HTTP 503"),
            "AI 后端能力探测失败：HTTP 503"
        );
        assert_eq!(
            ai_backend_reason(Locale::Zh, "custom provider error"),
            "custom provider error"
        );
        assert_eq!(
            ai_backend_fallback(Locale::Zh, "external agent disabled"),
            "AI 后端已按运行时策略切换。原因：外部 Agent 已禁用"
        );
    }
}
