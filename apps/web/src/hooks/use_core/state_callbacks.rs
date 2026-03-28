use super::callbacks::{
    DocCallbacks, MiscCallbacks, SourceControlCallbacks, SwitchCallbacks, SyncCallbacks,
};

pub(super) struct CoreStateCallbacks {
    pub(super) doc: DocCallbacks,
    pub(super) sync: SyncCallbacks,
    pub(super) sc: SourceControlCallbacks,
    pub(super) misc: MiscCallbacks,
    pub(super) switch: SwitchCallbacks,
}

impl CoreStateCallbacks {
    pub(super) fn new(
        doc: DocCallbacks,
        sync: SyncCallbacks,
        sc: SourceControlCallbacks,
        misc: MiscCallbacks,
        switch: SwitchCallbacks,
    ) -> Self {
        Self {
            doc,
            sync,
            sc,
            misc,
            switch,
        }
    }
}
