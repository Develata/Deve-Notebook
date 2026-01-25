use similar::{DiffTag, TextDiff};

/// View model for a single line in the diff
#[derive(Clone, Debug, PartialEq)]
pub struct LineView {
    pub num: Option<usize>,
    pub content: String,
    pub class: &'static str,
}

impl LineView {
    pub fn empty() -> Self {
        Self {
            num: None,
            content: "".to_string(),
            class: "bg-gray-100 dark:bg-[#252526]",
        }
    }
}

/// Compute side-by-side diff lines
pub fn compute_diff(old_content: &str, new_content: &str) -> (Vec<LineView>, Vec<LineView>) {
    let diff = TextDiff::from_lines(old_content, new_content);
    let mut left_lines = Vec::new();
    let mut right_lines = Vec::new();

    for op in diff.ops() {
        match op.tag() {
            DiffTag::Equal => {
                let old_range = op.old_range();
                let new_range = op.new_range();
                for (i, j) in old_range.zip(new_range) {
                    let content = old_content.lines().nth(i).unwrap_or("");
                    left_lines.push(LineView {
                        num: Some(i + 1),
                        content: content.to_string(),
                        class: "",
                    });
                    right_lines.push(LineView {
                        num: Some(j + 1),
                        content: content.to_string(),
                        class: "",
                    });
                }
            }
            DiffTag::Delete => {
                for i in op.old_range() {
                    let content = old_content.lines().nth(i).unwrap_or("");
                    left_lines.push(LineView {
                        num: Some(i + 1),
                        content: content.to_string(),
                        class: "bg-red-100 dark:bg-[#4b1818]",
                    });
                    right_lines.push(LineView::empty());
                }
            }
            DiffTag::Insert => {
                for j in op.new_range() {
                    let content = new_content.lines().nth(j).unwrap_or("");
                    left_lines.push(LineView::empty());
                    right_lines.push(LineView {
                        num: Some(j + 1),
                        content: content.to_string(),
                        class: "bg-green-100 dark:bg-[#143d20]",
                    });
                }
            }
            DiffTag::Replace => {
                for i in op.old_range() {
                    let content = old_content.lines().nth(i).unwrap_or("");
                    left_lines.push(LineView {
                        num: Some(i + 1),
                        content: content.to_string(),
                        class: "bg-red-100 dark:bg-[#4b1818]",
                    });
                    right_lines.push(LineView::empty());
                }
                for j in op.new_range() {
                    let content = new_content.lines().nth(j).unwrap_or("");
                    left_lines.push(LineView::empty());
                    right_lines.push(LineView {
                        num: Some(j + 1),
                        content: content.to_string(),
                        class: "bg-green-100 dark:bg-[#143d20]",
                    });
                }
            }
        }
    }

    (left_lines, right_lines)
}
