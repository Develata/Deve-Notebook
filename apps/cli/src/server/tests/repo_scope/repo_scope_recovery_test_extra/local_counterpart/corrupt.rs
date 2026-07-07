//! plan_ref:
//!   - 04_repository#repo-scope-runtime

use super::support::{build_state, remote_scope};
use crate::server::repo_scope::resolve_local_counterpart_repo;
use deve_core::codec;
use deve_core::ledger::{
    REDB_SCHEMA_VERSION, REPO_INFO_METADATA_KEY, REPO_METADATA, REPO_SCHEMA_VERSION_METADATA_KEY,
    RepoInfo, RepoManager,
};
use deve_core::models::PeerId;

#[test]
fn resolve_local_counterpart_repo_fails_closed_on_duplicate_local_url_matches() -> anyhow::Result<()>
{
    let (dir, state, _default_id, _test_id) = build_state()?;
    let mut mirror = RepoManager::init(
        dir.path().join("ledger"),
        10,
        Some("mirror"),
        Some("urn:mirror"),
    )?;
    mirror.set_projection_base_for_all_local_repos_checked(dir.path().join("notes"))?;
    let mirror_db = mirror.open_database(None, "mirror")?.db;
    let mirror_info = mirror
        .get_repo_info_for(None, Some("mirror"))?
        .expect("mirror info");
    let txn = mirror_db.begin_write()?;
    txn.open_table(REPO_METADATA)?.insert(
        &REPO_INFO_METADATA_KEY,
        codec::encode(&RepoInfo {
            uuid: mirror_info.uuid,
            name: "mirror".into(),
            url: Some("urn:test".into()),
        })?
        .as_slice(),
    )?;
    txn.commit()?;

    let peer_id = PeerId::new("peer-a");
    let remote_repo_id = uuid::Uuid::new_v4();
    state.repo.ensure_shadow_repo_info(
        &peer_id,
        &deve_core::ledger::RepoInfo {
            uuid: remote_repo_id,
            name: "shadow-test".into(),
            url: Some("urn:test".into()),
        },
    )?;

    let err = resolve_local_counterpart_repo(
        &state,
        &remote_scope(remote_repo_id, "shadow-test", peer_id),
    )
    .expect_err("duplicate local URL owners must fail closed");
    assert!(
        err.to_string()
            .contains("duplicate local repository URL urn:test")
    );
    Ok(())
}

#[test]
fn find_local_repo_name_by_url_fails_closed_when_candidate_metadata_is_unreadable()
-> anyhow::Result<()> {
    let (_dir, state, _default_id, _test_id) = build_state()?;
    let db = state.repo.open_database(None, "default")?.db;
    let txn = db.begin_write()?;
    {
        let mut table = txn.open_table(REPO_METADATA)?;
        let version = codec::encode(&REDB_SCHEMA_VERSION)?;
        table.insert(&REPO_SCHEMA_VERSION_METADATA_KEY, version.as_slice())?;
        table.insert(&REPO_INFO_METADATA_KEY, [0_u8, 1, 2, 3].as_slice())?;
    }
    txn.commit()?;

    let err = state
        .repo
        .find_local_repo_name_by_url("urn:default")
        .expect_err("broken local repo metadata must fail closed");
    assert!(
        err.to_string().contains("decode")
            || err.to_string().contains("deserialize")
            || err.to_string().contains("deserialization")
            || err.to_string().contains("postcard")
            || err.to_string().contains("unexpected end")
    );
    Ok(())
}
