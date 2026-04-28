# P1 Path Normalization 状态 - 2026-04-28

## 结果

- Path normalization cleanup 已完成。
- 本批次只收口 runtime / app 边界中的 forward-slash normalization，不改变已持久化路径语义。

## 已实现

- `crates/core/src/plugin/manifest.rs`
  - plugin capability path normalization 改为使用 `deve_core::utils::path::path_to_forward_slash`。
  - 保留原有词法 `.` / `..` 规约和 Windows drive-prefix 语义。
- `apps/cli/src/server/handlers/docs/copy_utils.rs`
  - docs copy 相对路径转换改为使用 `path_to_forward_slash`。
  - 增加 mixed separator 回归测试。
- `apps/web/src/components/search_box/file_ops/path_utils.rs`
  - `normalize_doc_path`、`validate_doc_shell_path`、`finalize_dst`、`collect_dirs` 改为使用 `to_forward_slash` / `path_to_forward_slash`。
  - 增加 Web file-op path helper 回归测试。

## 明确豁免

- `crates/core/src/utils/path.rs` 内部仍使用 `replace('\\', "/")`，这是统一工具函数本身的实现点。
- `crates/core/src/plugin/runtime/host/fs.rs` 中的 `replace('\\', "\\\\")` 是 Rhai 测试脚本文字串转义，不是 forward-slash path normalization。
- `apps/cli/build.rs` 的构建期 asset path helper 不进入 runtime ledger / DB / cache 权威路径；当前不为此引入 build-dependency。

## 验证

- `cargo fmt`
- `cargo fmt --check`
- `git diff --check`
- `scripts/check-acceptance-bindings.sh`
- `scripts/plan-coverage.sh --summary-missing-plan-ref`
- `rg -n -F "replace('\\\\', \"/\")" crates apps -g '!apps/web/js/**'`
- `rg -n -F "replace(\"\\\\\", \"/\")" crates apps -g '!apps/web/js/**'`
- `cargo test -p deve_core capability -- --nocapture`
- `cargo test -p deve_cli copy_utils -- --nocapture`
- `cargo test -p deve_web path_utils -- --nocapture`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test`
