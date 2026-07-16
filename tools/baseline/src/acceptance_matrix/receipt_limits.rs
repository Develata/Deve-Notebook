//! Shared receipt/claims resource limits for producers, collectors, and tag-ready.
//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

use anyhow::{Context, Result, bail};
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

pub(super) const MAX_RECEIPT_FILES: usize = 4_096;
pub(super) const MAX_EXECUTION_RECEIPTS: usize = 64;
pub(super) const MAX_RECEIPT_BYTES: u64 = 1024 * 1024;
pub(super) const MAX_TOTAL_RECEIPT_BYTES: u64 = 16 * 1024 * 1024;

pub(super) fn validate_file_size(label: &str, bytes: u64) -> Result<()> {
    if bytes > MAX_RECEIPT_BYTES {
        bail!("{label} exceeds {MAX_RECEIPT_BYTES} bytes");
    }
    Ok(())
}

pub(super) fn add_total_bytes(label: &str, total: &mut u64, bytes: u64) -> Result<()> {
    let next = total
        .checked_add(bytes)
        .with_context(|| format!("{label} byte count overflowed"))?;
    if next > MAX_TOTAL_RECEIPT_BYTES {
        bail!("{label} exceeds {MAX_TOTAL_RECEIPT_BYTES} bytes");
    }
    *total = next;
    Ok(())
}

pub(super) fn read_json_bounded(path: &Path, label: &str) -> Result<Vec<u8>> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("failed to inspect {label} {}", path.display()))?;
    if !metadata.file_type().is_file() {
        bail!("{label} is not a regular file: {}", path.display());
    }
    validate_file_size(label, metadata.len())?;
    let file =
        File::open(path).with_context(|| format!("failed to open {label} {}", path.display()))?;
    let mut content = Vec::with_capacity(metadata.len() as usize);
    file.take(MAX_RECEIPT_BYTES + 1)
        .read_to_end(&mut content)
        .with_context(|| format!("failed to read {label} {}", path.display()))?;
    validate_file_size(label, content.len() as u64)?;
    Ok(content)
}

#[cfg(test)]
mod tests {
    use super::{MAX_RECEIPT_BYTES, add_total_bytes, read_json_bounded};
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn bounded_json_read_rejects_oversized_metadata_before_allocation() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("deve-oversized-claims-{unique}.json"));
        let file = fs::File::create(&path).unwrap();
        file.set_len(MAX_RECEIPT_BYTES + 1).unwrap();

        assert!(read_json_bounded(&path, "claims JSON").is_err());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn aggregate_budget_rejects_the_first_over_limit_byte() {
        let mut total = 16 * 1024 * 1024;
        assert!(add_total_bytes("receipt group", &mut total, 1).is_err());
    }
}
