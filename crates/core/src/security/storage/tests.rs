use super::{load_or_generate_identity_key_at, load_or_generate_repo_key_at};

#[test]
fn identity_key_generates_on_missing_file() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let key = load_or_generate_identity_key_at(dir.path())?;
    assert_eq!(key.to_bytes().len(), 32);
    assert!(dir.path().join("identity.key").exists());
    Ok(())
}

#[test]
fn corrupt_identity_key_fails_closed() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("identity.key");
    std::fs::write(&path, [1, 2, 3])?;

    let err = match load_or_generate_identity_key_at(dir.path()) {
        Ok(_) => panic!("must fail closed"),
        Err(err) => err,
    };
    assert!(err.to_string().contains("Corrupt identity key"));
    assert_eq!(std::fs::read(path)?, vec![1, 2, 3]);
    Ok(())
}

#[test]
fn repo_key_generates_on_missing_file() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let key = load_or_generate_repo_key_at(dir.path())?;
    assert_eq!(key.to_bytes().len(), 32);
    assert!(dir.path().join("repo.key").exists());
    Ok(())
}

#[test]
fn corrupt_repo_key_fails_closed() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("repo.key");
    std::fs::write(&path, [9, 8, 7])?;

    let err = match load_or_generate_repo_key_at(dir.path()) {
        Ok(_) => panic!("must fail closed"),
        Err(err) => err,
    };
    assert!(err.to_string().contains("Corrupt repo key"));
    assert_eq!(std::fs::read(path)?, vec![9, 8, 7]);
    Ok(())
}

#[test]
fn concurrent_identity_initializers_return_the_persisted_key() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let root = std::sync::Arc::new(dir.path().to_path_buf());
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
    let workers = (0..8)
        .map(|_| {
            let root = root.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                load_or_generate_identity_key_at(root.as_ref()).map(|key| key.to_bytes())
            })
        })
        .collect::<Vec<_>>();
    let values = workers
        .into_iter()
        .map(|worker| worker.join().expect("identity worker"))
        .collect::<anyhow::Result<Vec<_>>>()?;
    let persisted = std::fs::read(dir.path().join("identity.key"))?;

    assert!(values.iter().all(|value| value.as_slice() == persisted));
    Ok(())
}

#[test]
fn concurrent_repo_initializers_return_the_persisted_key() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let root = std::sync::Arc::new(dir.path().to_path_buf());
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
    let workers = (0..8)
        .map(|_| {
            let root = root.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                load_or_generate_repo_key_at(root.as_ref()).map(|key| key.to_bytes())
            })
        })
        .collect::<Vec<_>>();
    let values = workers
        .into_iter()
        .map(|worker| worker.join().expect("repo worker"))
        .collect::<anyhow::Result<Vec<_>>>()?;
    let persisted = std::fs::read(dir.path().join("repo.key"))?;

    assert!(values.iter().all(|value| value.as_slice() == persisted));
    Ok(())
}

#[cfg(unix)]
#[test]
fn generated_key_files_are_owner_only_from_creation() -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir()?;
    load_or_generate_identity_key_at(dir.path())?;
    load_or_generate_repo_key_at(dir.path())?;

    for key in ["identity.key", "repo.key"] {
        let mode = std::fs::metadata(dir.path().join(key))?
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600, "{key}");
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn key_initialization_rejects_symlink_targets() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let outside = dir.path().join("outside.key");
    std::fs::write(&outside, [7_u8; 32])?;
    std::os::unix::fs::symlink(&outside, dir.path().join("identity.key"))?;

    let error = match load_or_generate_identity_key_at(dir.path()) {
        Ok(_) => panic!("identity symlink must fail closed"),
        Err(error) => error,
    };

    assert_eq!(std::fs::read(outside)?, vec![7_u8; 32]);
    assert!(!error.to_string().is_empty());
    Ok(())
}
