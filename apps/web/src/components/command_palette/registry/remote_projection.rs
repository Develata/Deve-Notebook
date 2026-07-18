//! plan_ref:
//!   - 14_commands#command-palette-shortcuts
//!   - 05_diff_logic#remote-projection-transport
//!   - 09_web_thin_client_ledger#web-edit-intent
//!
//! Thin command entries for typed Remote Projection push intents.

use crate::components::command_palette::types::Command;
use crate::i18n::{Locale, t};
use crate::runtime::scope_client::ScopeClient;
use crate::runtime::session_client::SessionClient;
use deve_core::protocol::{
    ClientMessage, RemoteProjectionProvider, RemoteProjectionPushRequest, ScopeNonce,
};
use leptos::prelude::*;
use uuid::Uuid;

pub(super) fn remote_projection_commands(
    locale: Locale,
    set_show: WriteSignal<bool>,
) -> Vec<Command> {
    let enabled_when = (t::command_palette::remote_projection_backend_intent)(locale);
    let unavailable_reason = (t::command_palette::remote_projection_scope_unavailable)(locale);
    let group = (t::command_palette::group_remote_projection)(locale);
    let context = RemoteProjectionCommandContext {
        enabled_when,
        unavailable_reason,
        group,
        set_show,
        intent: RemoteProjectionPushIntent::capture(),
    };
    vec![
        remote_projection_command(
            "remote_projection.webdav.push",
            (t::command_palette::webdav_push)(locale),
            RemoteProjectionProvider::WebDav,
            context.clone(),
        ),
        remote_projection_command(
            "remote_projection.s3.push",
            (t::command_palette::s3_push)(locale),
            RemoteProjectionProvider::S3,
            context,
        ),
    ]
}

#[derive(Clone)]
struct RemoteProjectionCommandContext {
    enabled_when: &'static str,
    unavailable_reason: &'static str,
    group: &'static str,
    set_show: WriteSignal<bool>,
    intent: Option<RemoteProjectionPushIntent>,
}

#[derive(Clone)]
struct RemoteProjectionPushIntent {
    session: SessionClient,
    repo_id: deve_core::models::RepoId,
    branch: Option<deve_core::models::PeerId>,
    scope_nonce: u64,
}

impl RemoteProjectionPushIntent {
    fn capture() -> Option<Self> {
        let session = use_context::<SessionClient>()?;
        let scope = use_context::<ScopeClient>()?;
        Self::admit(
            session.clone(),
            scope.current_repo_id.get(),
            scope.active_branch.get(),
            scope.current_scope_nonce.get(),
            session.handshake_ready.get(),
            session.handshake_scope_nonce.get(),
        )
    }

    fn admit(
        session: SessionClient,
        current_repo_id: Option<String>,
        branch: Option<deve_core::models::PeerId>,
        scope_nonce: u64,
        handshake_ready: bool,
        handshake_scope_nonce: Option<u64>,
    ) -> Option<Self> {
        if !handshake_ready || scope_nonce == 0 || handshake_scope_nonce != Some(scope_nonce) {
            return None;
        }
        let repo_id = current_repo_id?.parse().ok()?;
        Some(Self {
            session,
            repo_id,
            branch,
            scope_nonce,
        })
    }

    fn send(&self, provider: RemoteProjectionProvider) {
        send_remote_projection_push_for_scope(
            &self.session,
            self.repo_id,
            self.branch.clone(),
            self.scope_nonce,
            provider,
        );
    }
}

fn remote_projection_command(
    id: &'static str,
    title: &'static str,
    provider: RemoteProjectionProvider,
    context: RemoteProjectionCommandContext,
) -> Command {
    let RemoteProjectionCommandContext {
        enabled_when,
        unavailable_reason,
        group,
        set_show,
        intent,
    } = context;
    let command = match intent {
        Some(intent) => Command::available(id, title, move || {
            intent.send(provider);
            set_show.set(false);
        }),
        None => Command::unavailable(id, title, unavailable_reason, move || {
            set_show.set(false);
        }),
    };
    command.with_group(group).with_enabled_when(enabled_when)
}

fn send_remote_projection_push_for_scope(
    session: &SessionClient,
    repo_id: deve_core::models::RepoId,
    branch: Option<deve_core::models::PeerId>,
    scope_nonce: u64,
    provider: RemoteProjectionProvider,
) {
    session.ws.send(ClientMessage::RemoteProjectionPush(
        RemoteProjectionPushRequest {
            request_id: Uuid::new_v4(),
            repo_id,
            branch,
            scope_nonce: ScopeNonce::new(scope_nonce),
            provider,
        },
    ));
}

#[cfg(test)]
mod tests;
