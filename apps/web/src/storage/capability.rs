//! plan_ref:
//!   - 03_storage/index#browser-storage-layering
//!
//! Browser identity capability facts and fail-closed write blockers.
//! Platform probing stays in `js_bridge`; this module owns only typed policy.

use serde::{Deserialize, Serialize};

/// Browser platform facts needed by the repo-scoped WebLightPeer identity.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageCapabilities {
    pub webcrypto: bool,
    pub indexed_db: bool,
    pub local_storage: bool,
    pub ed25519: bool,
    #[serde(default)]
    pub indexed_db_probe_error: Option<String>,
    #[serde(default)]
    pub ed25519_probe_error: Option<String>,
}

impl StorageCapabilities {
    /// Return the first deterministic blocker for browser writer identity.
    pub const fn identity_blocker(&self) -> Option<BrowserIdentityBlocker> {
        if !self.webcrypto {
            Some(BrowserIdentityBlocker::WebCryptoUnavailable)
        } else if self.indexed_db_probe_error.is_some() {
            Some(BrowserIdentityBlocker::CapabilityProbeFailed)
        } else if !self.indexed_db {
            Some(BrowserIdentityBlocker::IndexedDbUnavailable)
        } else if self.ed25519_probe_error.is_some() {
            Some(BrowserIdentityBlocker::CapabilityProbeFailed)
        } else if !self.ed25519 {
            Some(BrowserIdentityBlocker::Ed25519Unavailable)
        } else {
            None
        }
    }

    /// Map platform capability facts to the fail-closed sync state.
    pub fn degraded_mode(&self) -> Option<DegradedSyncMode> {
        self.identity_blocker().map(|blocker| {
            if blocker == BrowserIdentityBlocker::CapabilityProbeFailed {
                DegradedSyncMode::capability_probe_failed(
                    self.indexed_db_probe_error
                        .as_deref()
                        .or(self.ed25519_probe_error.as_deref())
                        .unwrap_or("unknown browser identity probe failure"),
                )
            } else {
                DegradedSyncMode::capability(blocker)
            }
        })
    }
}

/// Stable reason why browser writer identity cannot become ready.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrowserIdentityBlocker {
    WebCryptoUnavailable,
    IndexedDbUnavailable,
    Ed25519Unavailable,
    CapabilityProbeFailed,
    IdentityRecoveryFailed,
}

impl BrowserIdentityBlocker {
    /// Stable diagnostic marker for logs and target-host gates.
    pub const fn code(self) -> &'static str {
        match self {
            Self::WebCryptoUnavailable => "webcrypto_unavailable",
            Self::IndexedDbUnavailable => "indexeddb_unavailable",
            Self::Ed25519Unavailable => "ed25519_unavailable",
            Self::CapabilityProbeFailed => "capability_probe_failed",
            Self::IdentityRecoveryFailed => "identity_recovery_failed",
        }
    }
}

/// Browser identity is unavailable; reads remain allowed while writes fail closed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DegradedSyncMode {
    pub blocker: BrowserIdentityBlocker,
    detail: Option<String>,
}

impl DegradedSyncMode {
    pub const fn capability(blocker: BrowserIdentityBlocker) -> Self {
        Self {
            blocker,
            detail: None,
        }
    }

    pub fn capability_probe_failed(detail: impl Into<String>) -> Self {
        Self {
            blocker: BrowserIdentityBlocker::CapabilityProbeFailed,
            detail: Some(detail.into()),
        }
    }

    pub fn identity_recovery_failed(detail: impl Into<String>) -> Self {
        Self {
            blocker: BrowserIdentityBlocker::IdentityRecoveryFailed,
            detail: Some(detail.into()),
        }
    }

    /// Diagnostic detail is log-only; UI permission logic must use `blocker`.
    pub fn diagnostic(&self) -> String {
        match self.detail.as_deref() {
            Some(detail) => format!("{}: {detail}", self.blocker.code()),
            None => self.blocker.code().to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BrowserIdentityBlocker, StorageCapabilities};

    fn full_capabilities() -> StorageCapabilities {
        StorageCapabilities {
            webcrypto: true,
            indexed_db: true,
            local_storage: true,
            ed25519: true,
            indexed_db_probe_error: None,
            ed25519_probe_error: None,
        }
    }

    #[test]
    fn storage_capabilities_allow_full_sync_when_identity_stack_exists() {
        assert_eq!(full_capabilities().degraded_mode(), None);
    }

    #[test]
    fn store_011_storage_capabilities_degrade_to_read_only_without_identity_stack() {
        let mut caps = full_capabilities();
        caps.indexed_db = false;

        let mode = caps.degraded_mode().expect("missing IndexedDB degrades");
        assert_eq!(mode.blocker, BrowserIdentityBlocker::IndexedDbUnavailable);
        assert_eq!(mode.diagnostic(), "indexeddb_unavailable");
    }

    #[test]
    fn browser_identity_capability_reports_ed25519_as_typed_blocker() {
        let mut caps = full_capabilities();
        caps.ed25519 = false;

        let mode = caps.degraded_mode().expect("missing Ed25519 degrades");
        assert_eq!(mode.blocker, BrowserIdentityBlocker::Ed25519Unavailable);
    }

    #[test]
    fn browser_identity_capability_does_not_treat_local_storage_as_writer_identity() {
        let mut caps = full_capabilities();
        caps.local_storage = false;

        assert_eq!(caps.degraded_mode(), None);
    }

    #[test]
    fn browser_identity_capability_uses_deterministic_blocker_precedence() {
        let caps = StorageCapabilities::default();

        assert_eq!(
            caps.identity_blocker(),
            Some(BrowserIdentityBlocker::WebCryptoUnavailable)
        );
    }

    #[test]
    fn browser_identity_capability_runtime_errors_keep_typed_blockers() {
        let probe = super::DegradedSyncMode::capability_probe_failed("adapter failure");
        let recovery = super::DegradedSyncMode::identity_recovery_failed("database failure");

        assert_eq!(probe.blocker, BrowserIdentityBlocker::CapabilityProbeFailed);
        assert_eq!(
            recovery.blocker,
            BrowserIdentityBlocker::IdentityRecoveryFailed
        );
        assert_eq!(
            probe.diagnostic(),
            "capability_probe_failed: adapter failure"
        );
        assert_eq!(
            recovery.diagnostic(),
            "identity_recovery_failed: database failure"
        );
    }

    #[test]
    fn browser_identity_capability_distinguishes_probe_failure_from_unsupported_algorithm() {
        let mut caps = full_capabilities();
        caps.ed25519 = false;
        caps.ed25519_probe_error = Some("OperationError".to_string());

        let mode = caps.degraded_mode().expect("probe failure degrades");
        assert_eq!(mode.blocker, BrowserIdentityBlocker::CapabilityProbeFailed);
        assert_eq!(mode.diagnostic(), "capability_probe_failed: OperationError");
    }

    #[test]
    fn browser_identity_capability_distinguishes_indexeddb_failure_from_unavailability() {
        let mut caps = full_capabilities();
        caps.indexed_db = false;
        caps.indexed_db_probe_error = Some("AbortError".to_string());

        let mode = caps.degraded_mode().expect("probe failure degrades");
        assert_eq!(mode.blocker, BrowserIdentityBlocker::CapabilityProbeFailed);
        assert_eq!(mode.diagnostic(), "capability_probe_failed: AbortError");
    }
}
