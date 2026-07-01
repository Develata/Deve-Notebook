# Mobile Surface Switcher Acceptance

- case_id: UI-MOB-021
  title: Mobile document/diff surface switcher
  preconditions:
    - 页面处于移动端视口
    - 至少打开两个文档，并已有一个 source-control diff session
  steps:
    - run: scripts/check-mobile-baseline.sh
    - run: cargo test -p deve_web mobile_surface -- --nocapture
    - ui_click: "open_switcher"
    - ui_assert: mobile_surface_sheet_visible true
    - ui_click: "mobile_surface_document_row"
    - ui_assert: editor_visible true
    - ui_click: "open_switcher"
    - ui_click: "mobile_surface_diff_row"
    - ui_assert: element_visible ".diff-view-mobile"
    - ui_assert: mobile_diff_mode "unified"
    - ui_click: "close_diff"
  assertions:
    - cli_assert: mobile_surface_switcher_marker_bound true
    - cli_assert: mobile_surface_sheet_marker_bound true
    - cli_assert: mobile_surface_kind_label_bound true
    - cli_assert: mobile_surface_touch_targets_min_size_bound true
    - cli_assert: mobile_surface_diff_restore_bound true
    - cli_assert: mobile_surface_close_diff_keeps_source_control_state true
    - cli_assert: mobile_surface_close_inactive_diff_preserves_active_diff true
    - cli_assert: mobile_surface_accessible_labels_include_titles true
    - cli_assert: mobile_surface_sheet_dialog_semantics_bound true
    - cli_assert: mobile_surface_sheet_focus_trap_bound true
    - ui_assert: mobile_surface_sheet_escape_closes true
    - ui_assert: mobile_surface_summary_kind_visible true
    - ui_assert: staged_pending_commit_state_unchanged true
