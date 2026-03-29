use super::{
    RepoListScope, RequestMatch, ShadowListScope, accepts_system_or_matching_request,
    peer_branch_matches_scope, repo_list_matches_scope, shadow_list_matches_scope,
    string_branch_matches_scope,
};
use crate::hooks::use_core::PendingBranchTarget;
use deve_core::models::PeerId;

#[path = "message_scope_test_branch.rs"]
mod branch;
#[path = "message_scope_test_repo.rs"]
mod repo;
#[path = "message_scope_test_request.rs"]
mod request;
#[path = "message_scope_test_shadow.rs"]
mod shadow;
