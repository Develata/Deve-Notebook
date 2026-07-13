use super::*;

#[test]
fn diff_projection_references_source_content_by_byte_range() {
    let projection = compute_diff_projection("a\nold\nz".into(), "a\nnew\nz".into()).unwrap();
    let changed = projection
        .rows
        .iter()
        .find(|row| row.left.kind == DiffCellKind::Delete)
        .unwrap();
    assert_eq!(projection.cell_text(&changed.left, true), Some("old"));
    assert_eq!(projection.cell_text(&changed.right, false), Some("new"));
    assert!(!projection.base_content.is_empty());
}

#[test]
fn diff_projection_word_ranges_use_utf16() {
    let projection = compute_diff_projection("A😀 old".into(), "A😀 new".into()).unwrap();
    let row = projection
        .rows
        .iter()
        .find(|row| row.hunk_id.is_some())
        .unwrap();
    let left = row.left.word_ranges.first().unwrap();
    let right = row.right.word_ranges.first().unwrap();
    assert_eq!(*left, DiffTextRange { start: 4, end: 7 });
    assert_eq!(*right, DiffTextRange { start: 4, end: 7 });
}

#[test]
fn diff_projection_reports_backend_algorithm() {
    let myers = compute_diff_projection("left".into(), "right".into()).unwrap();
    assert_eq!(myers.algorithm, DiffAlgorithm::Myers);
    let patience = compute_diff_projection("a\nold\nz".into(), "a\nnew\nz".into()).unwrap();
    assert_eq!(patience.algorithm, DiffAlgorithm::PatienceMyers);
}

#[test]
fn diff_projection_crosses_legacy_300_line_boundary() {
    let base = (0..650)
        .map(|i| format!("line-{i}"))
        .collect::<Vec<_>>()
        .join("\n");
    let mut target_lines: Vec<_> = (0..650).map(|i| format!("line-{i}")).collect();
    target_lines[299] = "changed-299".into();
    target_lines[300] = "changed-300".into();
    let projection = compute_diff_projection(base, target_lines.join("\n")).unwrap();
    assert_eq!(projection.hunks.len(), 1);
    assert_eq!(projection.added_lines, 2);
    assert_eq!(projection.deleted_lines, 2);
}

#[test]
fn diff_projection_folds_include_3_5_8() {
    let base = (0..80)
        .map(|i| format!("line-{i}"))
        .collect::<Vec<_>>()
        .join("\n");
    let target = base.replace("line-40", "changed-40");
    let projection = compute_diff_projection(base, target).unwrap();
    for context in [3, 5, 8] {
        assert!(
            projection
                .folds
                .iter()
                .any(|fold| fold.context_lines == context)
        );
    }
    let context_three: Vec<_> = projection
        .folds
        .iter()
        .filter(|fold| fold.context_lines == 3)
        .map(|fold| (fold.row_start, fold.row_end, fold.fold_id.clone()))
        .collect();
    assert_eq!(context_three.len(), 2);
    assert_eq!((context_three[0].0, context_three[0].1), (0, 37));
    assert_eq!((context_three[1].0, context_three[1].1), (44, 80));
    let repeated = compute_diff_projection(
        (0..80)
            .map(|i| format!("line-{i}"))
            .collect::<Vec<_>>()
            .join("\n"),
        (0..80)
            .map(|i| {
                if i == 40 {
                    "changed-40".into()
                } else {
                    format!("line-{i}")
                }
            })
            .collect::<Vec<_>>()
            .join("\n"),
    )
    .unwrap();
    assert_eq!(projection.folds, repeated.folds);
}

#[test]
fn diff_projection_resource_limits_fail_closed() {
    let oversized = "x".repeat(MAX_DIFF_INPUT_BYTES + 1);
    assert!(matches!(
        compute_diff_projection(oversized, String::new()),
        Err(DiffProjectionError::InputBytes { .. })
    ));
    let too_many_lines = "\n".repeat(MAX_DIFF_INPUT_LINES + 1);
    assert!(matches!(
        compute_diff_projection(too_many_lines, String::new()),
        Err(DiffProjectionError::InputLines { .. })
    ));
}

#[test]
fn diff_projection_accepts_exact_input_boundaries() {
    let exact_bytes = "x".repeat(MAX_DIFF_INPUT_BYTES);
    assert!(compute_diff_projection(exact_bytes, String::new()).is_ok());

    let fifty_thousand_lines = std::iter::repeat_n("x", 50_000)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(compute_diff_projection(fifty_thousand_lines.clone(), fifty_thousand_lines).is_ok());
}

#[test]
fn diff_projection_output_limit_is_checked_without_encoding_a_frame() {
    let projection = DiffProjection {
        projection_id: "oversized".into(),
        algorithm: DiffAlgorithm::Myers,
        base_content: "x".repeat(MAX_DIFF_PROJECTION_BYTES + 1),
        target_content: String::new(),
        rows: Vec::new(),
        hunks: Vec::new(),
        folds: Vec::new(),
        added_lines: 0,
        deleted_lines: 0,
        compute_micros: 0,
    };
    assert!(matches!(
        ensure_projection_size(&projection),
        Err(DiffProjectionError::OutputBytes { .. })
    ));
}

#[test]
fn diff_projection_cancelled_before_publish() {
    assert_eq!(
        compute_diff_projection_cancellable("a".into(), "b".into(), &|| true),
        Err(DiffProjectionError::Cancelled)
    );
}

#[test]
fn diff_projection_cancels_during_input_scan() {
    let checks = std::cell::Cell::new(0usize);
    let content = "line\n".repeat(20_000);
    let result = compute_diff_projection_cancellable(content, String::new(), &|| {
        checks.set(checks.get() + 1);
        checks.get() > 4
    });
    assert_eq!(result, Err(DiffProjectionError::Cancelled));
}

#[test]
fn diff_projection_preserves_blank_lines_and_empty_documents() {
    let projection = compute_diff_projection(String::new(), "a\n\nb".into()).unwrap();
    assert_eq!(projection.rows.len(), 3);
    assert_eq!(
        projection.cell_text(&projection.rows[1].right, false),
        Some("")
    );
    assert_eq!(projection.added_lines, 3);
}

#[test]
fn diff_projection_treats_terminal_newline_as_a_real_empty_line() {
    let projection = compute_diff_projection("a".into(), "a\n".into()).unwrap();
    assert_eq!(projection.rows.len(), 2);
    assert_eq!(projection.added_lines, 1);
    assert_eq!(projection.deleted_lines, 0);
    let added = projection
        .rows
        .iter()
        .find(|row| row.right.kind == DiffCellKind::Add)
        .unwrap();
    assert_eq!(projection.cell_text(&added.right, false), Some(""));
}

#[test]
fn diff_projection_hunk_uses_empty_side_insertion_point() {
    let projection = compute_diff_projection("a\nb".into(), "x\na\nb".into()).unwrap();
    let hunk = projection.hunks.first().unwrap();
    assert_eq!(hunk.old_lines, DiffLineRange { start: 1, end: 1 });
    assert_eq!(hunk.new_lines, DiffLineRange { start: 1, end: 2 });
}
