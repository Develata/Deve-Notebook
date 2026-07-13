//! Typed diff projection wire model.
//! plan_ref:
//!   - 05_diff_logic#typed-diff-projection-contract

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiffAlgorithm {
    Myers,
    PatienceMyers,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiffCellKind {
    Context,
    Add,
    Delete,
    Empty,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffByteRange {
    pub start: u32,
    pub end: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffTextRange {
    pub start: u32,
    pub end: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffCellProjection {
    pub line_number: Option<u32>,
    pub byte_range: Option<DiffByteRange>,
    pub word_ranges: Vec<DiffTextRange>,
    pub kind: DiffCellKind,
}

impl DiffCellProjection {
    pub(crate) fn empty() -> Self {
        Self {
            line_number: None,
            byte_range: None,
            word_ranges: Vec::new(),
            kind: DiffCellKind::Empty,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffRowProjection {
    pub row_id: u32,
    pub left: DiffCellProjection,
    pub right: DiffCellProjection,
    pub hunk_id: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffLineRange {
    /// 1-based, half-open. Empty sides use the insertion point twice.
    pub start: u32,
    pub end: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffHunkProjection {
    pub hunk_id: u32,
    pub row_start: u32,
    pub row_end: u32,
    pub old_lines: DiffLineRange,
    pub new_lines: DiffLineRange,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffFoldRange {
    pub fold_id: String,
    pub row_start: u32,
    pub row_end: u32,
    pub context_lines: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffProjection {
    pub projection_id: String,
    pub algorithm: DiffAlgorithm,
    pub base_content: String,
    pub target_content: String,
    pub rows: Vec<DiffRowProjection>,
    pub hunks: Vec<DiffHunkProjection>,
    pub folds: Vec<DiffFoldRange>,
    pub added_lines: u32,
    pub deleted_lines: u32,
    pub compute_micros: u64,
}

impl DiffProjection {
    pub fn cell_text<'a>(&'a self, cell: &DiffCellProjection, left: bool) -> Option<&'a str> {
        let range = cell.byte_range?;
        let content = if left {
            &self.base_content
        } else {
            &self.target_content
        };
        content.get(range.start as usize..range.end as usize)
    }
}
