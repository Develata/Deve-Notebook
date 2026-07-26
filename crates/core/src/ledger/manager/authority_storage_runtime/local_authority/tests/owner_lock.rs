//! plan_ref:
//!   - 03_storage/authority#local-authority-owner-contract
//!
//! OS owner-lock release regressions.

use super::*;

#[test]
fn inherited_descriptor_does_not_extend_owner_lock_lifetime() -> anyhow::Result<()> {
    let (dir, runtime, repo_id) = new_runtime()?;
    let inherited_descriptor = {
        let slots = runtime
            .inner
            .slots
            .lock()
            .map_err(|_| LocalAuthorityError::Poisoned)?;
        let Some(RepoAuthoritySlot::Active {
            resources,
            generation: 1,
            ..
        }) = slots.get(&repo_id)
        else {
            panic!("new runtime must own the active authority slot");
        };
        resources.authority_lock.file().try_clone()?
    };

    drop(runtime);
    let reopened = LocalAuthorityRuntime::open_existing(dir.path(), repo_id)?;
    drop(inherited_descriptor);
    drop(reopened);
    Ok(())
}
