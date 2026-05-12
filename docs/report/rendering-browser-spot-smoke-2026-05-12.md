# Rendering Browser Spot Smoke - 2026-05-12

本报告记录 Chrome MCP 渲染点验。测试使用隔离数据根，不依赖 checked-in `ledger/` 或 `vault/`。

## Environment

- Backend: `DEVE_LEDGER_DIR=/tmp/deve-render-spot-*/ledger DEVE_VAULT_PATH=/tmp/deve-render-spot-*/vault cargo run -p deve_cli --features search --bin deve_cli -- serve --dev --port 3001`
- Frontend: `NO_COLOR=true trunk serve --address 127.0.0.1 --port 8080`
- Browser URL: `http://127.0.0.1:8080/`
- Data root: `/tmp/deve-render-spot-*`

## Results

Passed:

- Web shell reached `Ready`.
- Created `render-spot.md`.
- Typed a task item and clicked the rendered checkbox widget.
- Workspace source changed from `- [ ] task` to `- [x] task`.
- Created `math-spot.md`.
- Typed `$$a^2$$`.
- Browser exposed `.katex-display`; source file remained `$$a^2$$`.
- Opened Search in Ready state, submitted `?needle`, received `render-spot.md 全文匹配`, and opened the result.
- Bottom status remained `就绪`.
- Current-page console error/warn list was empty.

## Notes

- Typing a math block immediately after a task item triggered editor list continuation and produced `- [ ] $$a^2$$`. Math projection was therefore verified in a separate pure math document. This is input-mode test hygiene, not a rendering authority failure.
- Large-document non-ready search blocking is covered by `cargo test -p deve_web large_doc_search_gate -- --nocapture`; the browser smoke covered the after-ready path because the partial-load window is not stable enough for manual Chrome MCP timing.

## Verification

已运行：

- `cargo test -p deve_web large_doc_search_gate -- --nocapture`
- `scripts/check-large-doc-baseline.sh`
- `scripts/check-cli-settings-baseline.sh`
- `scripts/check-architecture-registry.sh`
- `scripts/plan-coverage.sh`

