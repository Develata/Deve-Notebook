use super::model::{CHUNK_SIZE, LINE_HEIGHT_PX, LineKind, UnifiedLine};

#[derive(Clone, Copy, PartialEq, Eq)]
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

        let chunk_height = (CHUNK_SIZE as i32) * LINE_HEIGHT_PX;
        let max_chunk = total_lines.saturating_sub(1) / CHUNK_SIZE;
        let focus = (scroll_top.max(0) / chunk_height.max(1)) as usize;
        let viewport_chunks = ((viewport_height.max(1) + chunk_height - 1) / chunk_height) as usize;
        let radius = viewport_chunks.max(1) + 1;
        let near_start = focus.saturating_sub(radius);
        let near_end = (focus + radius).min(max_chunk);
        Self {
            start_chunk: near_start,
            end_chunk: near_end,
        }
    }

    pub fn line_range(self, total_lines: usize) -> (usize, usize) {
        if total_lines == 0 {
            return (0, 0);
        }
        let start = self.start_chunk.saturating_mul(CHUNK_SIZE).min(total_lines);
        let mut end = (self.end_chunk + 1).saturating_mul(CHUNK_SIZE);
        if end > total_lines {
            end = total_lines;
        }
        (start, end)
    }

    pub fn spacer_before_px(self) -> i32 {
        (self.start_chunk.saturating_mul(CHUNK_SIZE) as i32) * LINE_HEIGHT_PX
    }

    pub fn spacer_after_px(self, total_lines: usize) -> i32 {
        let (_, end) = self.line_range(total_lines);
        ((total_lines.saturating_sub(end)) as i32) * LINE_HEIGHT_PX
    }
}

pub fn slice_lines(lines: &[UnifiedLine], window: ChunkWindow) -> Vec<UnifiedLine> {
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
