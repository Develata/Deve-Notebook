# Web API query encoding

日期：2026-05-14

## 结论

Web HTTP adapter 已统一 repo-scoped query component 编码，避免 `repo_id` 中的保留字符改变 query 结构。

## 变更

- 新增 `apps/web/src/api/query.rs`，提供无依赖 percent-encoding helper。
- `fetch_graph_projection` URL 构造改为编码 `repo_id`。
- `fetch_git_mirror_repair_review` URL 构造改为编码 `repo_id`。
- Graph 与 Source Control baseline 绑定编码 helper 与覆盖测试。

## 验证

- `cargo test -p deve_web query_component -- --nocapture`
- `cargo test -p deve_web graph_projection_url -- --nocapture`
- `cargo test -p deve_web repair_review_url -- --nocapture`
