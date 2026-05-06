//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!   - 03_rendering#large-document-runtime
//!
use super::segment::compute_segment;
use super::{DiffAlgorithm, LineView};

pub struct DiffChunkJob {
    old_lines: Vec<String>,
    new_lines: Vec<String>,
    chunk_size: usize,
    old_start: usize,
    drift: isize,
    left: Vec<LineView>,
    right: Vec<LineView>,
    patience_used: bool,
}

impl DiffChunkJob {
    pub fn new(old_content: String, new_content: String, chunk_size: usize) -> Self {
        let old_lines: Vec<String> = old_content.lines().map(str::to_owned).collect();
        let new_lines: Vec<String> = new_content.lines().map(str::to_owned).collect();
        let capacity = old_lines.len().max(new_lines.len());
        Self {
            old_lines,
            new_lines,
            chunk_size: chunk_size.max(1),
            old_start: 0,
            drift: 0,
            left: Vec::with_capacity(capacity),
            right: Vec::with_capacity(capacity),
            patience_used: false,
        }
    }

    pub fn step(&mut self) -> bool {
        if !self.has_more() {
            return true;
        }

        let old_end = (self.old_start + self.chunk_size).min(self.old_lines.len());
        let new_start = (self.old_start as isize + self.drift).max(0) as usize;
        let new_end = (new_start + self.chunk_size).min(self.new_lines.len());
        let old_refs: Vec<&str> = self.old_lines[self.old_start..old_end]
            .iter()
            .map(String::as_str)
            .collect();
        let new_refs: Vec<&str> = self.new_lines[new_start..new_end]
            .iter()
            .map(String::as_str)
            .collect();

        let (l, r, delta, segment_patience) =
            compute_segment(&old_refs, &new_refs, self.old_start, new_start);
        self.patience_used |= segment_patience;
        self.left.extend(l);
        self.right.extend(r);
        self.old_start = old_end;
        self.drift += delta;

        !self.has_more()
    }

    pub fn finish(self) -> ((Vec<LineView>, Vec<LineView>), DiffAlgorithm) {
        let algo = if self.patience_used {
            DiffAlgorithm::PatienceMyers
        } else {
            DiffAlgorithm::Myers
        };
        ((self.left, self.right), algo)
    }

    fn has_more(&self) -> bool {
        self.old_start < self.old_lines.len()
            || ((self.old_start as isize) + self.drift) < (self.new_lines.len() as isize)
    }
}

pub fn compute_diff_chunked_inner(
    old_content: &str,
    new_content: &str,
    chunk_size: usize,
) -> ((Vec<LineView>, Vec<LineView>), DiffAlgorithm) {
    let old_lines: Vec<&str> = old_content.lines().collect();
    let new_lines: Vec<&str> = new_content.lines().collect();
    let mut left = Vec::new();
    let mut right = Vec::new();
    let mut old_start = 0usize;
    let mut drift = 0isize;
    let mut patience_used = false;

    while old_start < old_lines.len() || ((old_start as isize) + drift) < (new_lines.len() as isize)
    {
        let old_end = (old_start + chunk_size).min(old_lines.len());
        let new_start = (old_start as isize + drift).max(0) as usize;
        let new_end = (new_start + chunk_size).min(new_lines.len());

        let (l, r, delta, segment_patience) = compute_segment(
            &old_lines[old_start..old_end],
            &new_lines[new_start..new_end],
            old_start,
            new_start,
        );
        patience_used |= segment_patience;
        left.extend(l);
        right.extend(r);
        old_start = old_end;
        drift += delta;

        if old_start >= old_lines.len() && new_end >= new_lines.len() {
            break;
        }
    }

    let algo = if patience_used {
        DiffAlgorithm::PatienceMyers
    } else {
        DiffAlgorithm::Myers
    };
    ((left, right), algo)
}

pub fn compute_diff_preview_inner(
    old_content: &str,
    new_content: &str,
    max_lines: usize,
) -> ((Vec<LineView>, Vec<LineView>), DiffAlgorithm) {
    let old_preview = old_content
        .lines()
        .take(max_lines)
        .collect::<Vec<_>>()
        .join("\n");
    let new_preview = new_content
        .lines()
        .take(max_lines)
        .collect::<Vec<_>>()
        .join("\n");
    compute_diff_chunked_inner(&old_preview, &new_preview, max_lines.max(1))
}

#[cfg(test)]
mod tests {
    use super::{DiffChunkJob, compute_diff_chunked_inner, compute_diff_preview_inner};

    #[test]
    fn diff_first_viewport_preview_bounds_initial_compute_rows() {
        let old_content = (0..3_000)
            .map(|i| format!("old-{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let new_content = (0..3_000)
            .map(|i| format!("new-{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let ((left, right), _) = compute_diff_preview_inner(&old_content, &new_content, 160);

        assert!(left.len() <= 320);
        assert_eq!(left.len(), right.len());
    }

    #[test]
    fn diff_first_viewport_chunk_job_matches_sync_chunked_output() {
        let old_content = "a\nb\nc\nd\ne\nf".to_string();
        let new_content = "a\nb2\nc\nd\ne2\nf\ng".to_string();
        let expected = compute_diff_chunked_inner(&old_content, &new_content, 2);
        let mut job = DiffChunkJob::new(old_content, new_content, 2);

        while !job.step() {}

        assert_eq!(job.finish(), expected);
    }

    #[test]
    fn diff_first_viewport_chunk_job_handles_insert_only_document() {
        let old_content = String::new();
        let new_content = "a\nb\nc\nd\ne".to_string();
        let expected = compute_diff_chunked_inner(&old_content, &new_content, 2);
        let mut job = DiffChunkJob::new(old_content, new_content, 2);

        while !job.step() {}

        assert_eq!(job.finish(), expected);
    }

    #[test]
    fn diff_first_viewport_chunk_job_handles_delete_only_document() {
        let old_content = "a\nb\nc\nd\ne".to_string();
        let new_content = String::new();
        let expected = compute_diff_chunked_inner(&old_content, &new_content, 2);
        let mut job = DiffChunkJob::new(old_content, new_content, 2);

        while !job.step() {}

        assert_eq!(job.finish(), expected);
    }
}
