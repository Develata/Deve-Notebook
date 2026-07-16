//! plan_ref:
//!   - 07_network#projection-recovery-contract
//!   - 09_web_thin_client_ledger#projection-recovery-coordinator
//!
use deve_core::models::{PeerId, RepoId};
use deve_core::protocol::{ProjectionRecoveryPlan, ProjectionRecoveryRequired};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectionRefreshWork {
    pub flight_id: u64,
    pub required: ProjectionRecoveryRequired,
    pub plan: ProjectionRecoveryPlan,
}

#[derive(Clone)]
struct ProjectionRefreshPending {
    required: ProjectionRecoveryRequired,
    plan: ProjectionRecoveryPlan,
}

struct ProjectionRefreshFlight {
    work: ProjectionRefreshWork,
    doc_list_request_id: Option<String>,
    source_control_request_id: Option<String>,
}

#[derive(Default)]
struct ProjectionRefreshCycle {
    scope: Option<ProjectionRefreshScope>,
    active: Option<ProjectionRefreshFlight>,
    trailing: Option<ProjectionRefreshPending>,
    next_flight_id: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectionRefreshScope {
    pub connection_epoch: u64,
    pub repo_id: Option<RepoId>,
    pub branch: Option<PeerId>,
    pub scope_nonce: u64,
    pub scope_switch_pending: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectionRefreshResponse {
    DocList,
    SourceControl,
}

/// Scope-local refresh barrier with one active request group and one bounded
/// trailing group. A flight can be retired explicitly by timeout or failure.
#[derive(Clone, Default)]
pub struct ProjectionRefreshCoordinator {
    cycle: Arc<Mutex<ProjectionRefreshCycle>>,
}

impl ProjectionRefreshCoordinator {
    pub fn enter_scope(&self, scope: ProjectionRefreshScope) {
        let mut cycle = self.lock_cycle();
        if cycle.scope.as_ref() != Some(&scope) {
            let next_flight_id = cycle.next_flight_id;
            *cycle = ProjectionRefreshCycle {
                scope: Some(scope),
                next_flight_id,
                ..ProjectionRefreshCycle::default()
            };
        }
    }

    pub fn begin(
        &self,
        required: ProjectionRecoveryRequired,
        plan: ProjectionRecoveryPlan,
    ) -> Option<ProjectionRefreshWork> {
        let mut cycle = self.lock_cycle();
        let pending = ProjectionRefreshPending { required, plan };
        if cycle.active.is_none() {
            return Some(activate(&mut cycle, pending));
        }
        merge_trailing_refresh(&mut cycle.trailing, pending);
        None
    }

    pub fn register_requests(
        &self,
        flight_id: u64,
        doc_list_request_id: Option<String>,
        source_control_request_id: Option<String>,
    ) -> Option<ProjectionRefreshWork> {
        let mut cycle = self.lock_cycle();
        let active = cycle.active.as_mut()?;
        if active.work.flight_id != flight_id {
            return None;
        }
        active.doc_list_request_id = doc_list_request_id;
        active.source_control_request_id = source_control_request_id;
        promote_if_complete(&mut cycle)
    }

    pub fn complete_response(
        &self,
        response: ProjectionRefreshResponse,
        request_id: &str,
    ) -> Option<ProjectionRefreshWork> {
        let mut cycle = self.lock_cycle();
        let active = cycle.active.as_mut()?;
        let expected = match response {
            ProjectionRefreshResponse::DocList => &mut active.doc_list_request_id,
            ProjectionRefreshResponse::SourceControl => &mut active.source_control_request_id,
        };
        if expected.as_deref() != Some(request_id) {
            return None;
        }
        *expected = None;
        promote_if_complete(&mut cycle)
    }

    #[cfg(any(test, target_arch = "wasm32"))]
    pub fn retire(&self, flight_id: u64) -> bool {
        let mut cycle = self.lock_cycle();
        if cycle
            .active
            .as_ref()
            .is_none_or(|active| active.work.flight_id != flight_id)
        {
            return false;
        }
        cycle.active = None;
        cycle.trailing = None;
        true
    }

    pub fn retire_active(&self) -> bool {
        let mut cycle = self.lock_cycle();
        if cycle.active.is_none() {
            return false;
        }
        cycle.active = None;
        cycle.trailing = None;
        true
    }

    #[cfg(test)]
    pub fn is_active(&self) -> bool {
        self.lock_cycle().active.is_some()
    }

    fn lock_cycle(&self) -> std::sync::MutexGuard<'_, ProjectionRefreshCycle> {
        self.cycle
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

fn activate(
    cycle: &mut ProjectionRefreshCycle,
    pending: ProjectionRefreshPending,
) -> ProjectionRefreshWork {
    cycle.next_flight_id = cycle.next_flight_id.wrapping_add(1).max(1);
    let work = ProjectionRefreshWork {
        flight_id: cycle.next_flight_id,
        required: pending.required,
        plan: pending.plan,
    };
    cycle.active = Some(ProjectionRefreshFlight {
        work: work.clone(),
        doc_list_request_id: None,
        source_control_request_id: None,
    });
    work
}

fn merge_trailing_refresh(
    trailing: &mut Option<ProjectionRefreshPending>,
    pending: ProjectionRefreshPending,
) {
    if let Some(trailing) = trailing {
        trailing.required = pending.required;
        trailing.plan.documents = pending.plan.documents;
        trailing.plan.refresh_doc_list |= pending.plan.refresh_doc_list;
        trailing.plan.refresh_source_control |= pending.plan.refresh_source_control;
        trailing.plan.refresh_external_changes |= pending.plan.refresh_external_changes;
    } else {
        *trailing = Some(pending);
    }
}

fn promote_if_complete(cycle: &mut ProjectionRefreshCycle) -> Option<ProjectionRefreshWork> {
    let active = cycle.active.as_ref()?;
    if active.doc_list_request_id.is_some() || active.source_control_request_id.is_some() {
        return None;
    }
    cycle.active = None;
    cycle
        .trailing
        .take()
        .map(|pending| activate(cycle, pending))
}
