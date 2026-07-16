//! plan_ref:
//!   - 03_storage/authority#repo-mutation-publication-gate
//!
//! Durable mutation outcome classification.

use super::MutationPublication;

#[derive(Debug)]
pub(crate) enum MutationExecution<T, E> {
    NotCommitted(E),
    Committed {
        value: T,
        publication: MutationPublication,
    },
    ProjectionDegraded {
        value: T,
        error: E,
        publication: MutationPublication,
    },
    CommittedPartial {
        error: E,
        publication: MutationPublication,
    },
}

impl<T, E> MutationExecution<T, E> {
    pub(crate) fn not_committed(error: E) -> Self {
        Self::NotCommitted(error)
    }

    pub(crate) fn committed(value: T, publication: MutationPublication) -> Self {
        Self::Committed { value, publication }
    }

    pub(crate) fn projection_degraded(
        value: T,
        error: E,
        publication: MutationPublication,
    ) -> Self {
        Self::ProjectionDegraded {
            value,
            error,
            publication,
        }
    }

    pub(crate) fn committed_partial(error: E, publication: MutationPublication) -> Self {
        Self::CommittedPartial { error, publication }
    }

    pub(super) fn publication(&self) -> Option<&MutationPublication> {
        match self {
            Self::NotCommitted(_) => None,
            Self::Committed { publication, .. }
            | Self::ProjectionDegraded { publication, .. }
            | Self::CommittedPartial { publication, .. } => Some(publication),
        }
    }
}
