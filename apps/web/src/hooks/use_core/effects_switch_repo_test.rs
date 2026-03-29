use super::{RepoSwitchSignals, handle_repo_switched};
use deve_core::models::DocId;
use leptos::prelude::*;
use uuid::Uuid;

#[path = "effects_switch_repo_test_accept.rs"]
mod accept;
#[path = "effects_switch_repo_test_ignore.rs"]
mod ignore;
