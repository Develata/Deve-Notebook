pub mod hunk_fold;
mod model_chunk;
mod myers_fallback;
mod patience;
mod replace_word;
mod segment;
pub mod split_fold;

pub const CHUNK_SIZE: usize = 300;
pub const LINE_HEIGHT_PX: i32 = 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiffAlgorithm {
    Myers,
    PatienceMyers,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LineView {
    pub num: Option<usize>,
    pub content: String,
    pub class: &'static str,
    pub word_ranges: Vec<(usize, usize)>,
    pub kind: LineKind,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UnifiedLine {
    pub num: Option<usize>,
    pub content: String,
    pub class: &'static str,
    pub word_ranges: Vec<(usize, usize)>,
    pub kind: LineKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineKind {
    Normal,
    Add,
    Del,
    Empty,
}

impl LineView {
    pub fn empty() -> Self {
        Self {
            num: None,
            content: String::new(),
            class: "bg-[var(--diff-line-empty)]",
            word_ranges: Vec::new(),
            kind: LineKind::Empty,
        }
    }
}

pub fn compute_diff_with_meta(
    old_content: &str,
    new_content: &str,
) -> ((Vec<LineView>, Vec<LineView>), DiffAlgorithm) {
    model_chunk::compute_diff_chunked_inner(old_content, new_content, CHUNK_SIZE)
}

pub fn to_unified(left: &[LineView], right: &[LineView]) -> Vec<UnifiedLine> {
    let mut lines = Vec::with_capacity(left.len().saturating_add(right.len()));
    for (l, r) in left.iter().zip(right.iter()) {
        if !l.content.is_empty()
            && !r.content.is_empty()
            && l.class.is_empty()
            && r.class.is_empty()
        {
            lines.push(UnifiedLine {
                num: r.num,
                content: format!("  {}", r.content),
                class: "",
                word_ranges: Vec::new(),
                kind: LineKind::Normal,
            });
            continue;
        }
        if !l.content.is_empty() {
            let ranges = l.word_ranges.iter().map(|(s, e)| (s + 2, e + 2)).collect();
            lines.push(UnifiedLine {
                num: l.num,
                content: format!("- {}", l.content),
                class: "bg-[var(--diff-line-del)]",
                word_ranges: ranges,
                kind: LineKind::Del,
            });
        }
        if !r.content.is_empty() {
            let ranges = r.word_ranges.iter().map(|(s, e)| (s + 2, e + 2)).collect();
            lines.push(UnifiedLine {
                num: r.num,
                content: format!("+ {}", r.content),
                class: "bg-[var(--diff-line-add)]",
                word_ranges: ranges,
                kind: LineKind::Add,
            });
        }
    }
    lines
}
