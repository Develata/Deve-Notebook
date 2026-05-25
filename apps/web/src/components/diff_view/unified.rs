//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 11_ui_design/01_web#web-layout-persistence
//!
use super::model::{LINE_HEIGHT_PX, LineKind, UnifiedLine};

pub const DIFF_VIEWPORT_CHUNK_SIZE: usize = 80;
pub const DIFF_VIEWPORT_PREFETCH_CHUNKS: usize = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChunkWindow {
    pub start_chunk: usize,
    pub end_chunk: usize,
}

impl ChunkWindow {
    pub fn from_viewport(total_lines: usize, scroll_top: i32, viewport_height: i32) -> Self {
        if total_lines == 0 {
            return Self {
                start_chunk: 0,
                end_chunk: 0,
            };
        }

        let chunk_height = (DIFF_VIEWPORT_CHUNK_SIZE as i32) * LINE_HEIGHT_PX;
        let max_chunk = total_lines.saturating_sub(1) / DIFF_VIEWPORT_CHUNK_SIZE;
        let first_visible_chunk =
            ((scroll_top.max(0) / chunk_height.max(1)) as usize).min(max_chunk);
        let viewport_chunks = ((viewport_height.max(1) + chunk_height - 1) / chunk_height) as usize;
        let prefetch_chunks = DIFF_VIEWPORT_PREFETCH_CHUNKS;
        let near_start = first_visible_chunk.saturating_sub(prefetch_chunks);
        let last_visible_chunk = first_visible_chunk
            .saturating_add(viewport_chunks.max(1).saturating_sub(1))
            .min(max_chunk);
        let near_end = last_visible_chunk
            .saturating_add(prefetch_chunks)
            .min(max_chunk);
        Self {
            start_chunk: near_start,
            end_chunk: near_end,
        }
    }

    pub fn line_range(self, total_lines: usize) -> (usize, usize) {
        if total_lines == 0 {
            return (0, 0);
        }
        let start = self
            .start_chunk
            .saturating_mul(DIFF_VIEWPORT_CHUNK_SIZE)
            .min(total_lines);
        let mut end = (self.end_chunk + 1).saturating_mul(DIFF_VIEWPORT_CHUNK_SIZE);
        if end > total_lines {
            end = total_lines;
        }
        (start, end)
    }

    pub fn spacer_before_px(self) -> i32 {
        (self.start_chunk.saturating_mul(DIFF_VIEWPORT_CHUNK_SIZE) as i32) * LINE_HEIGHT_PX
    }

    pub fn spacer_after_px(self, total_lines: usize) -> i32 {
        let (_, end) = self.line_range(total_lines);
        ((total_lines.saturating_sub(end)) as i32) * LINE_HEIGHT_PX
    }
}

pub fn slice_lines<T: Clone>(lines: &[T], window: ChunkWindow) -> Vec<T> {
    let (start, end) = window.line_range(lines.len());
    lines[start..end].to_vec()
}

pub fn hunk_rows(lines: &[UnifiedLine]) -> Vec<usize> {
    lines
        .iter()
        .enumerate()
        .filter_map(|(idx, l)| match l.kind {
            LineKind::Add | LineKind::Del => Some(idx),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        ChunkWindow, DIFF_VIEWPORT_CHUNK_SIZE, DIFF_VIEWPORT_PREFETCH_CHUNKS, LINE_HEIGHT_PX,
        hunk_rows, slice_lines,
    };
    use crate::components::diff_view::model::{LineKind, UnifiedLine};

    #[test]
    fn diff_first_viewport_initial_window_is_bounded_for_long_doc() {
        let window = ChunkWindow::from_viewport(3_000, 0, 600);
        let (start, end) = window.line_range(3_000);
        let rendered_lines = end - start;

        assert_eq!(start, 0);
        assert!(rendered_lines < 3_000);
        assert!(rendered_lines <= DIFF_VIEWPORT_CHUNK_SIZE * 2);
    }

    #[test]
    fn diff_first_viewport_middle_window_keeps_prefetch_on_both_sides() {
        let top = (DIFF_VIEWPORT_CHUNK_SIZE as i32) * LINE_HEIGHT_PX * 8;
        let window = ChunkWindow::from_viewport(3_000, top, 600);

        assert_eq!(window.start_chunk, 8 - DIFF_VIEWPORT_PREFETCH_CHUNKS);
        assert_eq!(window.end_chunk, 8 + DIFF_VIEWPORT_PREFETCH_CHUNKS);
    }

    #[test]
    fn diff_first_viewport_spacers_preserve_scroll_extent() {
        let total_lines = 3_000;
        let window = ChunkWindow::from_viewport(total_lines, 16_000, 600);
        let (start, end) = window.line_range(total_lines);
        let rendered_px = ((end - start) as i32) * LINE_HEIGHT_PX;
        let total_px =
            window.spacer_before_px() + rendered_px + window.spacer_after_px(total_lines);

        assert_eq!(total_px, (total_lines as i32) * LINE_HEIGHT_PX);
    }

    #[test]
    fn diff_first_viewport_clamps_stale_scroll_after_shorter_doc() {
        let total_lines = 100;
        let window = ChunkWindow::from_viewport(total_lines, 160_000, 600);
        let (start, end) = window.line_range(total_lines);
        let rendered_px = ((end - start) as i32) * LINE_HEIGHT_PX;
        let total_px =
            window.spacer_before_px() + rendered_px + window.spacer_after_px(total_lines);

        assert!(start <= end);
        assert!(end <= total_lines);
        assert_eq!(total_px, (total_lines as i32) * LINE_HEIGHT_PX);
    }

    #[test]
    fn diff_first_viewport_slice_only_returns_window_rows() {
        let rows: Vec<usize> = (0..3_000).collect();
        let window = ChunkWindow::from_viewport(rows.len(), 0, 600);
        let visible = slice_lines(&rows, window);

        assert_eq!(visible.first(), Some(&0));
        assert_eq!(visible.len(), DIFF_VIEWPORT_CHUNK_SIZE * 2);
    }

    #[test]
    fn diff_hunk_rows_collect_changed_line_indices() {
        let lines = vec![
            UnifiedLine {
                num: Some(1),
                content: "same".to_string(),
                class: "",
                word_ranges: Vec::new(),
                kind: LineKind::Normal,
            },
            UnifiedLine {
                num: Some(2),
                content: "- old".to_string(),
                class: "",
                word_ranges: Vec::new(),
                kind: LineKind::Del,
            },
            UnifiedLine {
                num: Some(3),
                content: "+ new".to_string(),
                class: "",
                word_ranges: Vec::new(),
                kind: LineKind::Add,
            },
        ];

        assert_eq!(hunk_rows(&lines), vec![1, 2]);
    }
}
