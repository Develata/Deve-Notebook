use super::{RepoSwitchSignals, handle_repo_switched};
use crate::hooks::use_core::PendingRepoSwitch;
use deve_core::models::DocId;
use leptos::prelude::*;
use uuid::Uuid;

mod accept;
mod ignore;
