//! Backend-owned typed diff projection.
//! plan_ref:
//!   - 05_diff_logic#typed-diff-projection-contract
//!   - 10_rendering#large-document-runtime

mod algorithm;
mod error;
mod lines;
mod structure;
mod types;

use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};

pub use error::DiffProjectionError;
pub use types::{
    DiffAlgorithm, DiffByteRange, DiffCellKind, DiffCellProjection, DiffFoldRange,
    DiffHunkProjection, DiffLineRange, DiffProjection, DiffRowProjection, DiffTextRange,
};

pub const MAX_DIFF_INPUT_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_DIFF_INPUT_LINES: usize = 100_000;
pub const MAX_DIFF_PROJECTION_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_DIFF_COMPUTE_TIME: Duration = Duration::from_secs(5);

pub fn projection_wire_size(projection: &DiffProjection) -> Result<usize, DiffProjectionError> {
    postcard::experimental::serialized_size(projection)
        .map_err(|_| DiffProjectionError::Invariant("projection serialization"))
}

pub fn compute_diff_projection(
    base_content: String,
    target_content: String,
) -> Result<DiffProjection, DiffProjectionError> {
    compute_diff_projection_cancellable(base_content, target_content, &|| false)
}

pub fn compute_diff_projection_cancellable(
    base_content: String,
    target_content: String,
    cancelled: &dyn Fn() -> bool,
) -> Result<DiffProjection, DiffProjectionError> {
    let started = Instant::now();
    let input_bytes = base_content.len().saturating_add(target_content.len());
    if input_bytes > MAX_DIFF_INPUT_BYTES {
        return Err(DiffProjectionError::InputBytes {
            actual: input_bytes,
            limit: MAX_DIFF_INPUT_BYTES,
        });
    }
    let base_line_count = lines::line_count(&base_content, cancelled)?;
    let target_line_count = lines::line_count(&target_content, cancelled)?;
    let input_lines = base_line_count.saturating_add(target_line_count);
    if input_lines > MAX_DIFF_INPUT_LINES {
        return Err(DiffProjectionError::InputLines {
            actual: input_lines,
            limit: MAX_DIFF_INPUT_LINES,
        });
    }
    let base_lines = lines::line_spans(&base_content, base_line_count);
    let target_lines = lines::line_spans(&target_content, target_line_count);
    let deadline = started + MAX_DIFF_COMPUTE_TIME;
    let mut built = algorithm::build_rows(&base_lines, &target_lines, deadline, cancelled)?;
    let hunks = structure::attach_hunks(&mut built.rows, cancelled)?;
    if cancelled() {
        return Err(DiffProjectionError::Cancelled);
    }
    let folds = structure::build_folds(&built.rows, cancelled)?;
    if cancelled() {
        return Err(DiffProjectionError::Cancelled);
    }
    if Instant::now() >= deadline {
        return Err(DiffProjectionError::ComputeDeadline);
    }
    let projection_id = projection_id(&base_content, &target_content, built.algorithm);
    let projection = DiffProjection {
        projection_id,
        algorithm: built.algorithm,
        base_content,
        target_content,
        rows: built.rows,
        hunks,
        folds,
        added_lines: built.added,
        deleted_lines: built.deleted,
        compute_micros: started.elapsed().as_micros().min(u64::MAX as u128) as u64,
    };
    if cancelled() {
        return Err(DiffProjectionError::Cancelled);
    }
    if Instant::now() >= deadline {
        return Err(DiffProjectionError::ComputeDeadline);
    }
    ensure_projection_size(&projection)?;
    if cancelled() {
        return Err(DiffProjectionError::Cancelled);
    }
    Ok(projection)
}

fn ensure_projection_size(projection: &DiffProjection) -> Result<usize, DiffProjectionError> {
    let encoded_size = projection_wire_size(projection)?;
    if encoded_size > MAX_DIFF_PROJECTION_BYTES {
        Err(DiffProjectionError::OutputBytes {
            actual: encoded_size,
            limit: MAX_DIFF_PROJECTION_BYTES,
        })
    } else {
        Ok(encoded_size)
    }
}

fn projection_id(base: &str, target: &str, algorithm: DiffAlgorithm) -> String {
    let mut hash = Sha256::new();
    hash.update([algorithm as u8]);
    hash.update((base.len() as u64).to_le_bytes());
    hash.update(base.as_bytes());
    hash.update((target.len() as u64).to_le_bytes());
    hash.update(target.as_bytes());
    hex::encode(hash.finalize())
}

#[cfg(test)]
mod tests;
