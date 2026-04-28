# 代码审查基线 2026-04-28

> 本报告是 `docs/report/` 当前最新的全局代码审查基线。若与
> `docs/plan/`、operation specs、acceptance cases 或当前代码冲突，以后者为准。

## 总结

当前项目骨架已经明显干净于 2026-04-08 的旧 gap 报告描述。核心权威路径、repo-scoped
server runtime、Web thin client 写入门禁、Source Control 身份路径、Search baseline、
Settings 当前边界、Native AI 最小能力、Graph 只读投影、i18n facade 与 release runtime
smoke 路径均已实现或明确降级为 future。

剩余风险不再是“基础框架缺失”，而是合同精度与平台扩展：

- sync wire shape 仍使用 range request，而不是显式 `{ repo_id, known_vector }` 请求对象；
  snapshot 消息也没有命名的 `server_vector` 字段。
- WebCrypto / IndexedDB 当前更接近 capability probe 与 metadata/vector store，还不是完整的浏览器私钥权威模型。
- mobile native、desktop native packaging、graph visualization rendering、relay trust boundary 与若干 rendering 细节仍属于 future/partial。
- `baseline-2026-04-08.md` 与 `gap-*-2026-04-08.md` 存在大量过时断言，不能继续作为 active TODO 使用。

本次审查为 targeted code/docs inspection，没有重新跑全量测试；最新通过的质量门槛以
`release-smoke-status-2026-04-28.md` 为准。

## 模块审查

### Core (`crates/core`)

状态：大体对齐，核心 authority 边界清晰。

已实现 / 对齐：

- Ledger-first authority 清晰：ledger schema、append validation、projection、node metadata、source-control side tables 与 repo manager 按职责拆分。
- Watcher 已实现：`sync/watcher/` 具备 backend trait、debounced events、pre-arm scan、repo-scoped dispatch 与 watcher tests。旧报告中“没有真实 watcher backend”的断言已过时。
- Source Control 有独立 core surface 与 ledger-manager 集成；diff/commit target 已能携带稳定 `doc_id`，stale path 处理已收紧，docless exact-delete legacy 边界已明确。
- Protocol frame 已版本化为 `DEVEWSF2`；缺失 binary magic 走结构化 decode error，legacy JSON 显式 gate。
- Graph projection 是只读派生层，不写 ledger、workspace、search 或 source-control state。
- Search feature-gated，并有 in-memory/on-disk service 边界。
- tracked 代码中没有产品 MCP runtime。MCP 在 plan/docs 中只保留为退役说明或 Chrome MCP 浏览器验收工具语义。

当前缺口 / 风险：

- `crates/core/src/sync/protocol.rs` 仍把 `SyncRequest` 建模为 `{ peer_id, repo_id, range }`；若 plan §05 继续要求显式 `known_vector`，这是实际 plan/code gap。
- Snapshot transfer 暴露 repo 与 ops，但没有显式 `server_vector` 字段。需要实现该字段，或修订 plan 说明 `SyncHello.vector` 是当前 vector carrier。
- path normalization 仍有手写 `replace('\\', "/")`，包括 `plugin/manifest.rs`。这是小但明确的跨平台路径卫生问题。
- soft-size warning 仍有若干项，但已由 `soft-size-audit-2026-04-27.md` 说明；当前没有 hard fuse violation。

### Server / CLI (`apps/cli`)

状态：模块化程度较好，主路径基本对齐 plan。

已实现 / 对齐：

- WebSocket 未授权握手现在返回结构化 JSON `AuthErrorResponse::new(code)`；旧报告中的 plain `"Unauthorized"` 断言已过时。
- WS runtime 已拆成 `ws/auth`、`ws/receive`、`ws/route`、`ws/send`，结构化 `ProtocolError { error }` 是目标路径。
- server writer readiness 已提升为 `repo_id + scope_nonce` gate，覆盖 `WriterIdentity`、session lookup 与 edit checks。
- Source Control server handler 已拆成 query、mutation、target resolution、presentation、staging、discard、conflict、commit 与 error facade。
- `agent_bridge/policy.rs` 默认关闭，要求 enabled、trusted 与绝对路径 `AGENT_CLI_PATH`；旧报告中的默认 `opencode` spawn 断言已过时。
- repair / node-check / repo health 已暴露 degraded state 与 fail-closed 行为，`/api/node/role` 已进入 release runtime smoke。
- Config 当前边界清晰：只承诺 `config.toml + config print/set`；server-backed Settings API 仍为 future。

当前缺口 / 风险：

- release 前仍值得补一批小安全项：identity key 权限、登录审计 `User-Agent`、生产 CORS origin 强约束、dev CORS warning 文案。
- Docker release smoke 被宿主 docker daemon 阻塞，不是代码失败；应继续作为环境阻塞项记录。
- `server/mod.rs` 已从旧报告的大文件问题变成紧凑 module index；后续应保持这个边界。

### Web / UI (`apps/web`)

状态：当前是可用的 Web thin-client shell，rendering/native 仍有明确缺口。

已实现 / 对齐：

- Auth state、session expiry、reconnect 与 writer readiness 分离；`WsService` 以 `repo_id + scope_nonce` 存储写入就绪状态。
- repo scope restore 是 UUID-first；switch nonce 相对当前 scope 单调增长。
- editor sync 从 confirmed history + pending overlay 重建；edit intent 携带 `doc_id`、`client_id`、`client_op_id`、`scope_nonce`。
- i18n locale detection 现在优先读取 `deve.ui.locale`，其次 `navigator.language`，最后 fallback `en-US`；旧报告中的 `Locale::default()` 断言已过时。
- i18n facade 基本收口；plan-coverage 记录的 i18n allowlist debt 已归零。
- Web responsive shell 已覆盖 desktop/mobile 布局、safe-area mobile UI、bottom-sheet gesture、disconnect overlay 与 degraded storage banner。
- Native AI Chat 已具备最小 PLAN/BUILD slash mode、当前 Markdown context 与受控 BUILD Apply；默认不开放 shell/MCP/tool loop。

当前缺口 / 风险：

- rendering plan 在源码层仍需重新验收：chat Markdown 有简单 code toolbar；editor hybrid rendering 细节要单独按 plan §03 判定 current/future。
- WebCrypto / IndexedDB 目前不是完整浏览器私钥 authority 模型；`extractable: false`、degraded SyncPush/write blocking 仍需明确实现或修订 plan。
- mobile 当前是 responsive Web shell，不是 full native offline app；native service startup、random loopback port、IPC fallback、secure storage、crash recovery、key rotation 均未实现。
- UI 中仍有 ad hoc utility colors 与 z-index。此项是 P2 设计债，不是 P0 blocker。

### Docs / Reports

状态：报告卫生已改善，但 active queue 必须保持短。

已实现 / 对齐：

- `docs/report/README.md` 已声明 report 是历史证据而非 live contract。
- `release-smoke-status-2026-04-28.md` 正确区分 code gate 与 Docker daemon blocker。
- `soft-size-audit-2026-04-27.md` 已记录放宽后的文件长度策略，避免机械拆分导致碎片化。

当前缺口 / 风险：

- `baseline-2026-04-08.md` 是有价值的历史材料，但其结论已混有大量过时项，应原地归档而不是继续编辑成当前真相。
- `gap-core/server/web/i18n-ui-2026-04-08.md` 是 raw old scan，只能作 forensic input，不应直接转成 TODO。
- `next-tasks.md` 之前膨胀成实现流水日志；当前应只保留短执行队列，完成证据迁移到 dated reports。

## 已过时断言

以下旧断言已被当前代码或后续报告覆盖：

- “没有 Watcher backend”已过时；`sync/watcher` 与 `notify` 依赖已存在。
- “WS handshake 返回 plain Unauthorized”已过时；WS auth 返回结构化 JSON。
- “Agent Bridge 默认 spawn `opencode`”已过时；trusted-cli 已 policy-gated。
- “Locale 只用 `Locale::default()`”已过时；已实现 stored/browser detection。
- “server/mod.rs 是大边界文件”已过时；现在是紧凑 module index。
- “MCP 是当前或 future 产品 runtime”已过时；MCP runtime 退役，不应重新引入。

## 下一优先级

1. P0：关闭 sync vector wire contract。实现显式 `known_vector/server_vector`，或修订 plan §05 接受当前 `SyncHello.vector + range request` 设计。
2. P0：完成 browser storage authority boundary。覆盖 WebCrypto key generation、IndexedDB 可用性语义、degraded read-only 与 SyncPush/write blocking。
3. P1：补小安全批次。覆盖 key-file permissions、login audit fields、production CORS origin、dev CORS warning wording。
4. P1：清理 path normalization 偏离。优先 core/server/web boundary wrappers，不改变已存储路径语义。
5. P1/P2：把 rendering plan 拆成 current acceptance 与 future editor hybrid-rendering。
6. P3-10：把 Desktop/Mobile native adapter 从文档边界推进到 decision-complete 的实现计划。
7. P3-13：Graph visualization 继续保持 read-only projection 消费者，不反向写 authority state。

## 建议验证门槛

下一批实现优先跑 targeted gates：

```bash
scripts/check-network-baseline.sh
scripts/check-auth-baseline.sh
scripts/check-ai-baseline.sh
scripts/check-search-baseline.sh
scripts/check-release-baseline.sh
scripts/plan-coverage.sh --summary-missing-plan-ref
```

release tagging 前再跑完整门槛：

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo check --locked -p deve_web --target wasm32-unknown-unknown
cargo test --locked
```
