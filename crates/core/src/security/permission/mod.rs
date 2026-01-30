use serde::{Deserialize, Serialize};

/// Permission Action
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Allow,
    Deny,
    Ask,
}

/// A single permission rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub permission: String, // e.g., "fs_read", "net_connect"
    pub pattern: String,    // e.g., "/src/*.rs", "google.com", "*"
    pub action: Action,
}

/// A set of rules
pub type Ruleset = Vec<Rule>;

/// Permission Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub id: String,
    pub session_id: String,
    pub permission: String,
    pub patterns: Vec<String>,
    pub metadata: serde_json::Map<String, serde_json::Value>,
}

impl Rule {
    /// Check if this rule matches a request
    pub fn matches(&self, permission: &str, pattern: &str) -> bool {
        // Simple wildcard match for now (enhance with glob later)
        let perm_match = self.permission == "*" || self.permission == permission;
        let pattern_match =
            self.pattern == "*" || self.pattern == pattern || pattern.starts_with(&self.pattern);

        perm_match && pattern_match
    }
}

/// Evaluate a request against a ruleset
pub fn evaluate(permission: &str, pattern: &str, ruleset: &Ruleset) -> Action {
    // Strategy: Last match wins (like firewall rules usually, or opencode logic)
    // Opencode: "const match = merged.findLast(...)"

    for rule in ruleset.iter().rev() {
        if rule.matches(permission, pattern) {
            return rule.action.clone();
        }
    }

    // Default action: Ask (Safety first)
    Action::Ask
}
