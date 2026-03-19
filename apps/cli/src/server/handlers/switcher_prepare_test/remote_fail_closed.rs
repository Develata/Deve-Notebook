use super::{REPO_METADATA, RepoInfo, build_state, resolve_requested_repo_name};
use deve_core::models::PeerId;

#[test]
fn resolve_requested_repo_name_fails_closed_when_remote_display_name_is_ambiguous_even_with_uuid()
-> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let peer_id = PeerId::new("peer-remote");
    let peer_dir = state.repo.remotes_dir().join(peer_id.to_filename());
    std::fs::create_dir_all(&peer_dir)?;
    let first = RepoInfo {
        uuid: uuid::Uuid::new_v4(),
        name: "wiki".into(),
        url: Some("urn:test:wiki-a".into()),
    };
    let second = RepoInfo {
        uuid: uuid::Uuid::new_v4(),
        name: "wiki".into(),
        url: Some("urn:test:wiki-b".into()),
    };
    for (stem, info) in [("wiki-a", &first), ("wiki-b", &second)] {
        let db = redb::Database::create(peer_dir.join(format!("{stem}.redb")))?;
        let write = db.begin_write()?;
        write
            .open_table(REPO_METADATA)?
            .insert(&0, bincode::serialize(info)?.as_slice())?;
        write.commit()?;
    }

    let err = resolve_requested_repo_name(&state, Some(&peer_id), "wiki", Some(second.uuid))
        .expect_err("ambiguous remote display name must not fall back to repo id");
    assert!(
        err.to_string()
            .contains("ambiguous remote repository selector")
    );
    Ok(())
}
