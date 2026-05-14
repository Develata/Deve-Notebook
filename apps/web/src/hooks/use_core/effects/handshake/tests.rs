use super::state::{reset_handshake_attempt_state, suspended_handshake_mode_key};
use super::{
    handshake_mode_key, restore_bootstrap_key, should_reset_manual_retry,
    should_restore_session_scope, should_suspend_handshake,
};
use crate::hooks::use_core::{PendingBranchTarget, types::HandshakeSignals};
use crate::storage::DegradedSyncMode;
use crate::storage::identity::StoredPeerIdentity;
use deve_core::models::{PeerId, VersionVector};
use leptos::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

mod reset;
mod restore;
mod suspend;

#[test]
fn manual_retry_nonce_change_requests_handshake_reset() {
    assert!(!should_reset_manual_retry(3, 3));
    assert!(should_reset_manual_retry(3, 4));
}
