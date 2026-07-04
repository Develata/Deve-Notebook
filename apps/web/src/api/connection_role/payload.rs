//! plan_ref:
//!   - 18_release#runtime-observability
//!

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NodeRoleProbeResult {
    pub summary: String,
    pub source_control_authority: String,
    pub host_file_copy_absolute_path: bool,
    pub host_file_reveal_in_system_explorer: bool,
}

impl NodeRoleProbeResult {
    pub(super) fn from_json(json: &serde_json::Value) -> Option<Self> {
        is_node_role_payload(json).then(|| Self {
            summary: format_node_role_summary(json),
            source_control_authority: source_control_authority(json).to_string(),
            host_file_copy_absolute_path: host_file_action_bool(json, "copy_absolute_path"),
            host_file_reveal_in_system_explorer: host_file_action_bool(
                json,
                "reveal_in_system_explorer",
            ),
        })
    }
}

pub(crate) fn http_base_from_ws_url(ws_url: &str) -> String {
    let http_url = match ws_url.strip_prefix("wss://") {
        Some(rest) => format!("https://{rest}"),
        None => match ws_url.strip_prefix("ws://") {
            Some(rest) => format!("http://{rest}"),
            None => ws_url.to_string(),
        },
    };
    strip_ws_path_suffix(&http_url)
}

fn strip_ws_path_suffix(http_url: &str) -> String {
    let split_idx = http_url.find(['?', '#']).unwrap_or(http_url.len());
    let base = &http_url[..split_idx];
    match base.strip_suffix("/ws") {
        Some(base) => base.to_string(),
        None => base.to_string(),
    }
}

pub(super) fn node_role_url_for_http_base(http_base: &str) -> String {
    let split_idx = http_base.find(['?', '#']).unwrap_or(http_base.len());
    let http_url = http_base[..split_idx].trim_end_matches('/');
    format!("{}/api/node/role", http_url)
}

pub(super) fn format_node_role_summary(json: &serde_json::Value) -> String {
    let role = str_field(json, "role", "unknown");
    let main_port = json.get("main_port").and_then(|v| v.as_u64()).unwrap_or(0);
    let ws_port = json.get("ws_port").and_then(|v| v.as_u64()).unwrap_or(0);
    let version = str_field(json, "version", "unknown-version");
    let profile = str_field(json, "profile", "unknown-profile");
    let delivery = str_field(json, "delivery", "unknown-delivery");
    let environment = str_field(json, "environment", "unknown-env");
    let repo_health = format_repo_health(json);
    let source_control = format_source_control(json);

    let role_text = if role == "proxy" && main_port > 0 {
        format!("proxy -> {} (ws:{})", main_port, ws_port)
    } else if ws_port > 0 {
        format!("{} (ws:{})", role, ws_port)
    } else {
        role.to_string()
    };
    format!(
        "{} | v{} | {} | {} | {} | repos:{} | {}",
        role_text, version, profile, delivery, environment, repo_health, source_control
    )
}

fn str_field<'a>(json: &'a serde_json::Value, key: &str, fallback: &'a str) -> &'a str {
    json.get(key).and_then(|v| v.as_str()).unwrap_or(fallback)
}

fn is_node_role_payload(json: &serde_json::Value) -> bool {
    has_str_field(json, "role")
        && has_u64_field(json, "ws_port")
        && has_u64_field(json, "main_port")
        && has_str_field(json, "version")
        && has_str_field(json, "profile")
        && has_str_field(json, "delivery")
        && has_str_field(json, "environment")
        && json.get("repo_health").is_some_and(is_repo_health_payload)
        && json
            .get("source_control")
            .is_some_and(is_source_control_payload)
        && json
            .get("host_file_actions")
            .is_some_and(is_host_file_actions_payload)
}

fn format_repo_health(json: &serde_json::Value) -> String {
    let Some(repo_health) = json.get("repo_health") else {
        return "unknown".into();
    };
    let status = str_field(repo_health, "status", "unknown");
    let total = repo_health
        .get("local_total")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let degraded = repo_health
        .get("degraded")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    format!("{} ({}/{})", status, degraded, total)
}

fn format_source_control(json: &serde_json::Value) -> String {
    format!(
        "sc:{}/{}",
        source_control_authority(json),
        source_control_git_main_mirror(json)
    )
}

fn source_control_authority(json: &serde_json::Value) -> &str {
    let Some(source_control) = json.get("source_control") else {
        return "unknown";
    };
    normalize_source_control_authority(str_field(source_control, "authority", "unknown"))
}

fn source_control_git_main_mirror(json: &serde_json::Value) -> &str {
    let Some(source_control) = json.get("source_control") else {
        return "unknown";
    };
    normalize_git_main_mirror(str_field(source_control, "git_main_mirror", "unknown"))
}

fn host_file_action_bool(json: &serde_json::Value, key: &str) -> bool {
    json.get("host_file_actions")
        .and_then(|actions| actions.get(key))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

fn is_repo_health_payload(json: &serde_json::Value) -> bool {
    has_str_field(json, "status")
        && has_u64_field(json, "local_total")
        && has_u64_field(json, "healthy")
        && has_u64_field(json, "degraded")
}

fn is_source_control_payload(json: &serde_json::Value) -> bool {
    let authority_ok = json
        .get("authority")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|authority| matches!(authority, "ngit" | "unknown"));
    let mirror_ok = json
        .get("git_main_mirror")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|mirror| matches!(mirror, "main" | "unknown"));
    authority_ok && mirror_ok
}

fn is_host_file_actions_payload(json: &serde_json::Value) -> bool {
    json.get("copy_absolute_path")
        .and_then(serde_json::Value::as_bool)
        .is_some()
        && json
            .get("reveal_in_system_explorer")
            .and_then(serde_json::Value::as_bool)
            .is_some()
}

fn has_str_field(json: &serde_json::Value, key: &str) -> bool {
    json.get(key).and_then(serde_json::Value::as_str).is_some()
}

fn has_u64_field(json: &serde_json::Value, key: &str) -> bool {
    json.get(key).and_then(serde_json::Value::as_u64).is_some()
}

fn normalize_source_control_authority(mode: &str) -> &'static str {
    match mode {
        "ngit" => "ngit",
        "unknown" => "unknown",
        _ => "unknown",
    }
}

fn normalize_git_main_mirror(mirror: &str) -> &'static str {
    match mirror {
        "main" => "main",
        "unknown" => "unknown",
        _ => "unknown",
    }
}
