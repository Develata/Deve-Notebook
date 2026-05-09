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
