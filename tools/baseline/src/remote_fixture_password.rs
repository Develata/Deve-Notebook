//! Narrow Argon2id password hasher for target-host RemoteBrowser fixtures.
//! plan_ref:
//!   - 18_release#remote-browser-candidate-fixture

use anyhow::{Context, Result, bail};
use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString},
};
use rand_core::OsRng;
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

const MAX_PASSWORD_BYTES: u64 = 4096;

pub(crate) fn run(args: &[String]) -> Result<()> {
    let [flag, path] = args else {
        bail!("remote-fixture-password-hash: expected --password-file <path>");
    };
    if flag != "--password-file" {
        bail!("remote-fixture-password-hash: expected --password-file <path>");
    }
    let phc = hash_password_file(Path::new(path))?;
    println!("{phc}");
    Ok(())
}

fn hash_password_file(path: &Path) -> Result<String> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("inspect password file {}", path.display()))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || is_reparse(&metadata) {
        bail!("remote-fixture-password-hash: password input must be a regular non-link file");
    }
    if metadata.len() == 0 || metadata.len() > MAX_PASSWORD_BYTES {
        bail!(
            "remote-fixture-password-hash: password input size is outside 1..={MAX_PASSWORD_BYTES}"
        );
    }
    let file =
        File::open(path).with_context(|| format!("open password file {}", path.display()))?;
    let opened_metadata = file
        .metadata()
        .with_context(|| format!("inspect opened password file {}", path.display()))?;
    if !opened_metadata.is_file() || is_reparse(&opened_metadata) {
        bail!("remote-fixture-password-hash: password input must remain a regular non-link file");
    }
    let mut password = Vec::with_capacity((metadata.len().min(MAX_PASSWORD_BYTES) + 1) as usize);
    if let Err(error) = file.take(MAX_PASSWORD_BYTES + 1).read_to_end(&mut password) {
        password.fill(0);
        return Err(error).with_context(|| format!("read password file {}", path.display()));
    }
    if password.len() as u64 > MAX_PASSWORD_BYTES {
        password.fill(0);
        bail!(
            "remote-fixture-password-hash: password input size is outside 1..={MAX_PASSWORD_BYTES}"
        );
    }
    if password.last() == Some(&b'\n') {
        password.pop();
        if password.last() == Some(&b'\r') {
            password.pop();
        }
    }
    if password.is_empty() || password.contains(&0) {
        password.fill(0);
        bail!("remote-fixture-password-hash: password must be non-empty and contain no NUL");
    }
    let salt = SaltString::generate(&mut OsRng);
    let result = Argon2::default()
        .hash_password(&password, &salt)
        .map(|phc| phc.to_string())
        .map_err(|error| anyhow::anyhow!("remote-fixture-password-hash: hashing failed: {error}"));
    password.fill(0);
    result
}

fn is_reparse(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x0400 != 0
    }
    #[cfg(not(windows))]
    {
        let _ = metadata;
        false
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_PASSWORD_BYTES, hash_password_file};
    use std::{fs, path::PathBuf};

    #[test]
    fn hashes_password_file_without_echoing_plaintext() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "deve-remote-fixture-password-{}-{nonce}.txt",
            std::process::id(),
        ));
        fs::write(&path, b"fixture-password\n").expect("write password fixture");
        let phc = hash_password_file(&path).expect("hash password");
        assert!(phc.starts_with("$argon2id$"));
        assert!(!phc.contains("fixture-password"));
        fs::remove_file(path).expect("remove password fixture");
    }

    #[test]
    fn rejects_nul_and_oversized_inputs() {
        let nul = temp_path("nul");
        fs::write(&nul, b"secret\0suffix").expect("write NUL fixture");
        assert!(hash_password_file(&nul).is_err());
        fs::remove_file(&nul).expect("remove NUL fixture");

        let oversized = temp_path("oversized");
        fs::write(&oversized, vec![b'x'; MAX_PASSWORD_BYTES as usize + 1])
            .expect("write oversized fixture");
        assert!(hash_password_file(&oversized).is_err());
        fs::remove_file(&oversized).expect("remove oversized fixture");
    }

    #[test]
    fn rejects_symlink_when_host_allows_creation() {
        let target = temp_path("target");
        let link = temp_path("link");
        fs::write(&target, b"secret").expect("write target");
        if !create_symlink(&target, &link) {
            fs::remove_file(target).expect("remove target");
            return;
        }
        assert!(hash_password_file(&link).is_err());
        fs::remove_file(link).expect("remove link");
        fs::remove_file(target).expect("remove target");
    }

    fn temp_path(label: &str) -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "deve-remote-fixture-password-{label}-{}-{nonce}",
            std::process::id()
        ))
    }

    #[cfg(unix)]
    fn create_symlink(target: &std::path::Path, link: &std::path::Path) -> bool {
        std::os::unix::fs::symlink(target, link).is_ok()
    }

    #[cfg(windows)]
    fn create_symlink(target: &std::path::Path, link: &std::path::Path) -> bool {
        std::os::windows::fs::symlink_file(target, link).is_ok()
    }
}
