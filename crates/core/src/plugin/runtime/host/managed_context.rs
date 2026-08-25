//! plan_ref:
//!   - 19_plugins#plugin-runtime-boundary
//!
//! Runtime-scoped managed mutation hosts. The context belongs to one plugin
//! runtime generation; a thread-local execution scope prevents overlapping
//! native backend generations from sharing process-global authority adapters.

use super::{ManagedNoteMutationHost, ManagedSourceControlMutationHost};
use anyhow::{Context, Result};
use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::Arc;

pub struct PluginHostContext {
    note: Arc<dyn ManagedNoteMutationHost>,
    source_control: Arc<dyn ManagedSourceControlMutationHost>,
}

impl PluginHostContext {
    pub fn new(
        note: Arc<dyn ManagedNoteMutationHost>,
        source_control: Arc<dyn ManagedSourceControlMutationHost>,
    ) -> Self {
        Self {
            note,
            source_control,
        }
    }
}

thread_local! {
    static ACTIVE_CONTEXTS: RefCell<Vec<Arc<PluginHostContext>>> = const { RefCell::new(Vec::new()) };
}

pub struct PluginHostContextScope {
    context: Arc<PluginHostContext>,
    _thread_bound: PhantomData<Rc<()>>,
}

impl PluginHostContextScope {
    pub fn enter(context: Arc<PluginHostContext>) -> Self {
        ACTIVE_CONTEXTS.with(|contexts| contexts.borrow_mut().push(context.clone()));
        Self {
            context,
            _thread_bound: PhantomData,
        }
    }
}

impl Drop for PluginHostContextScope {
    fn drop(&mut self) {
        ACTIVE_CONTEXTS.with(|contexts| {
            let popped = contexts.borrow_mut().pop();
            debug_assert!(
                popped
                    .as_ref()
                    .is_some_and(|context| Arc::ptr_eq(context, &self.context)),
                "plugin host execution scopes must retire in LIFO order"
            );
        });
    }
}

pub(super) fn managed_note_mutation_host() -> Result<Arc<dyn ManagedNoteMutationHost>> {
    ACTIVE_CONTEXTS.with(|contexts| {
        contexts
            .borrow()
            .last()
            .map(|context| context.note.clone())
            .context("ManagedNoteMutationHost not configured for this plugin runtime")
    })
}

pub(super) fn managed_source_control_mutation_host()
-> Result<Arc<dyn ManagedSourceControlMutationHost>> {
    ACTIVE_CONTEXTS.with(|contexts| {
        contexts
            .borrow()
            .last()
            .map(|context| context.source_control.clone())
            .context("ManagedSourceControlMutationHost not configured for this plugin runtime")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::runtime::host::{
        ManagedNoteWriteIntent, ManagedSourceControlCommitIntent, ManagedSourceControlStageIntent,
    };
    use crate::source_control::CommitInfo;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct NoteProbe(AtomicUsize);

    impl ManagedNoteMutationHost for NoteProbe {
        fn write_managed_note(&self, _intent: ManagedNoteWriteIntent) -> Result<()> {
            self.0.fetch_add(1, Ordering::AcqRel);
            Ok(())
        }
    }

    struct SourceControlProbe;

    impl ManagedSourceControlMutationHost for SourceControlProbe {
        fn stage_source_control(&self, _intent: ManagedSourceControlStageIntent) -> Result<()> {
            Ok(())
        }

        fn commit_source_control(
            &self,
            _intent: ManagedSourceControlCommitIntent,
        ) -> Result<CommitInfo> {
            anyhow::bail!("source-control probe is not called by this test")
        }
    }

    fn context(note: Arc<NoteProbe>) -> Arc<PluginHostContext> {
        Arc::new(PluginHostContext::new(note, Arc::new(SourceControlProbe)))
    }

    fn write_probe() {
        managed_note_mutation_host()
            .expect("runtime-scoped note host")
            .write_managed_note(ManagedNoteWriteIntent {
                repo_name: "repo".to_string(),
                repo_path: "note.md".to_string(),
                content: "probe".to_string(),
            })
            .expect("probe write");
    }

    #[test]
    fn overlapping_runtime_scopes_keep_managed_hosts_isolated() {
        let first = Arc::new(NoteProbe(AtomicUsize::new(0)));
        let second = Arc::new(NoteProbe(AtomicUsize::new(0)));
        let first_scope = PluginHostContextScope::enter(context(first.clone()));
        write_probe();
        {
            let _second_scope = PluginHostContextScope::enter(context(second.clone()));
            write_probe();
        }
        write_probe();
        drop(first_scope);

        assert_eq!(first.0.load(Ordering::Acquire), 2);
        assert_eq!(second.0.load(Ordering::Acquire), 1);
        assert!(managed_note_mutation_host().is_err());
    }
}
