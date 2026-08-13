//! plan_ref:
//!   - 15_settings#native-ai-provider-settings
//!
//! Strict allowlist parser and atomic persistence for `<data-root>/ai.env`.

use super::ProviderSettingsSnapshot;
use super::source::{default_snapshot, validate_snapshot};
use anyhow::{Context, Result, anyhow, bail};
use deve_core::utils::fs::{
    HostPathIdentity, HostPathKind, create_atomic_replace_temp, open_regular_file_read,
    replace_file_atomically, sync_directory,
};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(1);
const KEYS: [&str; 5] = [
    "AI_PROVIDER",
    "AI_BASE_URL",
    "AI_API_KEY",
    "AI_MODEL",
    "AI_MAX_TOKENS",
];

pub(super) struct AiEnvStore {
    root: HostPathIdentity,
    target: PathBuf,
}

pub(super) struct PersistFailure {
    pub(super) after_publish: bool,
    pub(super) _error: anyhow::Error,
}

impl AiEnvStore {
    pub(super) fn new(data_root: &Path) -> Result<Self> {
        let canonical = std::fs::canonicalize(data_root).context("canonicalize data root")?;
        let root = HostPathIdentity::capture(&canonical, HostPathKind::Directory)
            .context("capture data root identity")?;
        Ok(Self {
            target: canonical.join("ai.env"),
            root,
        })
    }

    pub(super) fn load(&self) -> Result<Option<ProviderSettingsSnapshot>> {
        self.ensure_root()?;
        let file = match open_regular_file_read(&self.target, "Native AI settings") {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error).context("open Native AI settings"),
        };
        let mut text = String::new();
        file.take(65_537)
            .read_to_string(&mut text)
            .context("read Native AI settings")?;
        if text.len() > 65_536 {
            bail!("Native AI settings file exceeds size budget");
        }
        let mut values = BTreeMap::new();
        for entry in dotenvy::from_read_iter(text.as_bytes()) {
            let (key, value) = entry.context("parse Native AI settings")?;
            if values.insert(key.clone(), value).is_some() {
                bail!("Native AI settings contains a duplicate key: {key}");
            }
        }
        if values.keys().any(|key| !KEYS.contains(&key.as_str())) {
            bail!("Native AI settings contains an unsupported key");
        }
        let mut snapshot = default_snapshot();
        if let Some(value) = values.get("AI_PROVIDER") {
            snapshot.provider = serde_json::from_value(serde_json::Value::String(value.clone()))
                .context("invalid AI_PROVIDER in Native AI settings")?;
        }
        if let Some(value) = values.get("AI_BASE_URL") {
            snapshot.base_url = value.clone();
        }
        if let Some(value) = values.get("AI_API_KEY") {
            snapshot.api_key = value.clone();
        }
        if let Some(value) = values.get("AI_MODEL") {
            snapshot.model = value.clone();
        }
        if let Some(value) = values.get("AI_MAX_TOKENS") {
            snapshot.max_tokens = value.parse().context("invalid AI_MAX_TOKENS")?;
        }
        validate_snapshot(&snapshot)?;
        Ok(Some(snapshot))
    }

    pub(super) fn persist(
        &self,
        snapshot: &ProviderSettingsSnapshot,
    ) -> Result<(), PersistFailure> {
        self.persist_inner(snapshot)
            .map_err(|(after_publish, error)| PersistFailure {
                after_publish,
                _error: error,
            })
    }

    fn persist_inner(
        &self,
        snapshot: &ProviderSettingsSnapshot,
    ) -> Result<(), (bool, anyhow::Error)> {
        self.ensure_root().map_err(|error| (false, error))?;
        match open_regular_file_read(&self.target, "Native AI settings") {
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err((
                    false,
                    anyhow::Error::new(error).context("validate existing Native AI settings"),
                ));
            }
        }
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temp = self
            .target
            .with_file_name(format!(".ai.env.tmp-{}-{sequence}", std::process::id()));
        let mut file = create_atomic_replace_temp(&temp)
            .context("create Native AI settings temp")
            .map_err(|error| (false, error))?;
        let mut published = false;
        let result = (|| -> Result<()> {
            #[cfg(unix)]
            file.set_permissions(std::os::unix::fs::PermissionsExt::from_mode(0o600))
                .context("set Native AI settings permissions")?;
            file.write_all(serialize(snapshot).as_bytes())
                .context("write Native AI settings")?;
            file.sync_all().context("sync Native AI settings")?;
            self.ensure_root()?;
            replace_file_atomically(&file, &temp, &self.target)
                .context("replace Native AI settings")?;
            published = true;
            #[cfg(test)]
            if self.root.path().join(".ai-env-fail-after-replace").exists() {
                bail!("injected Native AI settings failure after atomic replace");
            }
            sync_directory(self.root.path()).context("sync Native AI settings directory")?;
            Ok(())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&temp);
        }
        result.map_err(|error| (published, error))
    }

    fn ensure_root(&self) -> Result<()> {
        if !self.root.revalidate().context("revalidate data root")? {
            return Err(anyhow!("Native AI settings data root identity changed"));
        }
        Ok(())
    }
}

fn serialize(snapshot: &ProviderSettingsSnapshot) -> String {
    let provider = serde_json::to_value(snapshot.provider)
        .ok()
        .and_then(|value| value.as_str().map(ToOwned::to_owned))
        .unwrap_or_else(|| "openai-chat-completions".to_string());
    format!(
        "AI_PROVIDER={}\nAI_BASE_URL={}\nAI_API_KEY={}\nAI_MODEL={}\nAI_MAX_TOKENS={}\n",
        quote(&provider),
        quote(&snapshot.base_url),
        quote(&snapshot.api_key),
        quote(&snapshot.model),
        snapshot.max_tokens
    )
}

fn quote(value: &str) -> String {
    let escaped = value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('$', "\\$");
    format!("\"{escaped}\"")
}
