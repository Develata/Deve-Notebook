//! plan_ref:
//!   - 15_settings#native-ai-provider-settings
//!   - 16_ai_agent#native-ai-chat-runtime
//!
//! Server-owned Native AI provider settings authority.

mod file;
pub(crate) mod http;
mod registry;
mod source;

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, RwLock};

pub(crate) use registry::{ProviderSettingsRegistration, current, register};

use source::{default_snapshot, snapshot_from_environment, validate_snapshot};

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ProviderProtocol {
    #[default]
    OpenaiChatCompletions,
    OpenaiResponses,
    AnthropicMessages,
}

impl ProviderProtocol {
    pub(crate) const fn endpoint_suffix(self) -> &'static str {
        match self {
            Self::OpenaiChatCompletions => "chat/completions",
            Self::OpenaiResponses => "responses",
            Self::AnthropicMessages => "messages",
        }
    }
}

#[derive(Clone)]
pub(crate) struct ProviderSettingsSnapshot {
    pub(crate) provider: ProviderProtocol,
    pub(crate) base_url: String,
    pub(crate) api_key: String,
    pub(crate) model: String,
    pub(crate) max_tokens: u32,
    pub(crate) revision: u64,
}

impl ProviderSettingsSnapshot {
    pub(crate) fn endpoint(&self) -> String {
        format!(
            "{}/{}",
            self.base_url.trim().trim_end_matches('/'),
            self.provider.endpoint_suffix()
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SettingsSource {
    Defaults,
    UiManaged,
    Environment,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct ProviderSettingsProjection {
    pub(crate) provider: ProviderProtocol,
    pub(crate) base_url: String,
    pub(crate) model: String,
    pub(crate) max_tokens: u32,
    pub(crate) key_configured: bool,
    pub(crate) source: SettingsSource,
    pub(crate) revision: u64,
    pub(crate) writable: bool,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReplaceProviderSettings {
    pub(crate) expected_revision: u64,
    pub(crate) provider: ProviderProtocol,
    pub(crate) base_url: String,
    pub(crate) model: String,
    pub(crate) max_tokens: u32,
    #[serde(default)]
    pub(crate) api_key: Option<String>,
    #[serde(default)]
    pub(crate) clear_api_key: bool,
}

struct RuntimeState {
    snapshot: Arc<ProviderSettingsSnapshot>,
    source: SettingsSource,
    durability_uncertain: bool,
}

pub(crate) struct NativeAiProviderSettingsRuntime {
    state: RwLock<RuntimeState>,
    store: Option<file::AiEnvStore>,
}

impl NativeAiProviderSettingsRuntime {
    #[cfg(test)]
    pub(super) fn ready_for_test() -> Self {
        let mut snapshot = default_snapshot();
        snapshot.api_key = "fixture-secret".to_string();
        Self {
            state: RwLock::new(RuntimeState {
                snapshot: Arc::new(snapshot),
                source: SettingsSource::Defaults,
                durability_uncertain: false,
            }),
            store: None,
        }
    }

    pub(crate) fn from_data_root(data_root: &Path) -> Result<Self> {
        Self::from_sources(data_root, std::env::vars().collect())
    }

    pub(crate) fn environment_only() -> Result<Self> {
        let values = std::env::vars().collect::<BTreeMap<_, _>>();
        let (snapshot, source) = snapshot_from_environment(&values)?
            .unwrap_or_else(|| (default_snapshot(), SettingsSource::Defaults));
        Ok(Self {
            state: RwLock::new(RuntimeState {
                snapshot: Arc::new(snapshot),
                source,
                durability_uncertain: false,
            }),
            store: None,
        })
    }

    fn from_sources(data_root: &Path, values: BTreeMap<String, String>) -> Result<Self> {
        let store = file::AiEnvStore::new(data_root)?;
        let (snapshot, source) = match snapshot_from_environment(&values)? {
            Some(pair) => pair,
            None => match store.load()? {
                Some(snapshot) => (snapshot, SettingsSource::UiManaged),
                None => (default_snapshot(), SettingsSource::Defaults),
            },
        };
        Ok(Self {
            state: RwLock::new(RuntimeState {
                snapshot: Arc::new(snapshot),
                source,
                durability_uncertain: false,
            }),
            store: Some(store),
        })
    }

    pub(crate) fn snapshot(&self) -> Result<Arc<ProviderSettingsSnapshot>> {
        let state = self
            .state
            .read()
            .map_err(|_| anyhow!("AI provider settings lock poisoned"))?;
        if state.durability_uncertain {
            return Err(anyhow!("AI provider settings durability is uncertain"));
        }
        Ok(state.snapshot.clone())
    }

    pub(crate) fn projection(&self) -> Result<ProviderSettingsProjection> {
        let state = self
            .state
            .read()
            .map_err(|_| anyhow!("AI provider settings lock poisoned"))?;
        if state.durability_uncertain {
            return Err(anyhow!("AI provider settings durability is uncertain"));
        }
        Ok(project(&state.snapshot, state.source, self.store.is_some()))
    }

    pub(crate) fn replace(
        &self,
        request: ReplaceProviderSettings,
    ) -> Result<ProviderSettingsProjection, ReplaceError> {
        let mut state = self.state.write().map_err(|_| ReplaceError::Internal)?;
        if state.durability_uncertain {
            return Err(ReplaceError::Persistence);
        }
        if state.source == SettingsSource::Environment || self.store.is_none() {
            return Err(ReplaceError::EnvironmentManaged);
        }
        if request.expected_revision != state.snapshot.revision {
            return Err(ReplaceError::RevisionConflict);
        }
        if request.clear_api_key
            && request
                .api_key
                .as_deref()
                .is_some_and(|key| !key.is_empty())
        {
            return Err(ReplaceError::Invalid);
        }
        let api_key = if request.clear_api_key {
            String::new()
        } else if let Some(key) = request.api_key {
            if key.is_empty() {
                state.snapshot.api_key.clone()
            } else {
                key
            }
        } else {
            state.snapshot.api_key.clone()
        };
        let next_revision = state
            .snapshot
            .revision
            .checked_add(1)
            .ok_or(ReplaceError::Internal)?;
        let next = ProviderSettingsSnapshot {
            provider: request.provider,
            base_url: request.base_url.trim().to_string(),
            api_key,
            model: request.model.trim().to_string(),
            max_tokens: request.max_tokens,
            revision: next_revision,
        };
        validate_snapshot(&next).map_err(|_| ReplaceError::Invalid)?;
        if let Err(failure) = self
            .store
            .as_ref()
            .ok_or(ReplaceError::EnvironmentManaged)?
            .persist(&next)
        {
            if failure.after_publish {
                state.durability_uncertain = true;
            }
            tracing::error!(
                category = "native_ai_settings_persist_failed",
                after_publish = failure.after_publish,
                "Native AI settings persistence failed"
            );
            return Err(ReplaceError::Persistence);
        }
        state.snapshot = Arc::new(next);
        state.source = SettingsSource::UiManaged;
        Ok(project(&state.snapshot, state.source, true))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReplaceError {
    EnvironmentManaged,
    RevisionConflict,
    Invalid,
    Persistence,
    Internal,
}

fn project(
    snapshot: &ProviderSettingsSnapshot,
    source: SettingsSource,
    has_store: bool,
) -> ProviderSettingsProjection {
    ProviderSettingsProjection {
        provider: snapshot.provider,
        base_url: snapshot.base_url.clone(),
        model: snapshot.model.clone(),
        max_tokens: snapshot.max_tokens,
        key_configured: !snapshot.api_key.is_empty(),
        source,
        revision: snapshot.revision,
        writable: has_store && source != SettingsSource::Environment,
    }
}

#[cfg(test)]
mod tests;
