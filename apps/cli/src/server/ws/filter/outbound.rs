//! plan_ref:
//!   - 07_network#server-ws-runtime
//!
//! Broadcast filter outbound nonce stamping helpers.

use super::{BroadcastFilter, stamp};
#[cfg(test)]
use deve_core::protocol::ServerError;
use deve_core::protocol::{
    ProjectionRecoveryCause, ProjectionRecoveryPlan, ProjectionRecoveryRequired, ServerMessage,
};

impl BroadcastFilter {
    pub(crate) fn stamp_scope_nonce(&self, msg: ServerMessage) -> Option<ServerMessage> {
        let Some(scope) = &self.scope else {
            return Some(msg);
        };
        let Ok(scope) = scope.read() else {
            tracing::error!(
                "WS broadcast filter read lock poisoned during nonce stamp; dropping broadcast"
            );
            return None;
        };

        Some(stamp::stamp_scope_nonce(msg, scope.scope_nonce.get()))
    }

    #[cfg(test)]
    pub(crate) fn scoped_protocol_error(
        &self,
        error: ServerError,
        switch_nonce: Option<u64>,
    ) -> Option<ServerMessage> {
        let scope_nonce = match &self.scope {
            None => None,
            Some(scope) => match scope.read() {
                Ok(scope) => Some(scope.scope_nonce.get()),
                Err(_) => {
                    tracing::error!(
                        "WS broadcast filter read lock poisoned while building scoped protocol error"
                    );
                    return None;
                }
            },
        };
        Some(stamp::scoped_protocol_error(
            error,
            switch_nonce,
            scope_nonce,
        ))
    }

    pub(crate) fn scoped_broadcast_gap_recovery(&self, skipped: u64) -> Option<ServerMessage> {
        let scope = self.scope.as_ref()?;
        let scope = match scope.read() {
            Ok(scope) => scope,
            Err(_) => {
                tracing::error!(
                    "WS broadcast filter read lock poisoned while building gap recovery"
                );
                return None;
            }
        };
        let repo_id = scope.active_repo_id?;
        Some(ServerMessage::ProjectionRecoveryRequired(
            ProjectionRecoveryRequired {
                repo_id,
                branch: scope.active_branch.clone(),
                scope_nonce: Some(scope.scope_nonce.get()),
                cause: ProjectionRecoveryCause::BroadcastGap { skipped },
                plan: ProjectionRecoveryPlan::broadcast_gap(),
            },
        ))
    }
}
