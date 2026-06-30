# Mobile Surface Switcher Acceptance

- case_id: UI-MOB-021
  title: Mobile document/diff surface switcher
  preconditions:
    - 页面处于移动端视口
    - 至少打开两个文档，并已有一个 source-control diff session
  steps:
    - run: scripts/check-mobile-baseline.sh
    - run: cargo test -p deve_web mobile_surface -- --nocapture
    - ui_measure: "surface_switcher"
    - ui_assert: mobile_surface_type_label_visible true
    - ui_click: "open_switcher"
    - ui_assert: mobile_surface_sheet_visible true
    - ui_measure: "mobile_surface_rows"
    - ui_measure: "mobile_surface_close_buttons"
    - ui_open_drawer: "left"
    - ui_assert: mobile_surface_sheet_visible false
    - ui_click: "drawer_close_buttons"
    - ui_click: "open_switcher"
    - ui_click: "close_sheet"
    - ui_assert: mobile_surface_sheet_visible false
    - ui_focus: "open_switcher"
    - ui_keypress: "Enter"
    - ui_assert: mobile_surface_sheet_visible true
    - ui_keypress: "Enter"
    - ui_assert: mobile_surface_sheet_visible false
    - ui_click: "open_switcher"
    - ui_click: "mobile_surface_document_row"
    - ui_assert: editor_visible true
    - ui_click: "open_switcher"
    - ui_click: "mobile_surface_diff_row"
    - ui_assert: element_visible ".diff-view-mobile"
    - ui_assert: mobile_diff_mode "unified"
    - ui_click: "close_diff"
  assertions:
    - cli_assert: mobile_surface_switcher_marker_bound true
    - cli_assert: mobile_surface_type_label_marker_bound true
    - cli_assert: mobile_surface_sheet_marker_bound true
    - cli_assert: mobile_surface_runtime_transition_close_bound true
    - cli_assert: editor_tab_runtime_resets_on_branch_scope_change true
    - cli_assert: mobile_surface_drawer_close_bound true
    - cli_assert: mobile_surface_switcher_keyboard_toggle_bound true
    - cli_assert: mobile_surface_touch_targets_min_size_bound true
    - cli_assert: mobile_surface_diff_restore_bound true
    - cli_assert: mobile_surface_close_diff_keeps_source_control_state true
    - ui_assert: staged_pending_commit_state_unchanged true
