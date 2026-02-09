use super::segment::compute_segment;
use super::{DiffAlgorithm, LineView};

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
