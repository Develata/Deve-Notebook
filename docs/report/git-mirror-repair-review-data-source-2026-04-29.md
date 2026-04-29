# Git Mirror Repair Review Data Source - 2026-04-29

## 结论

- Git mirror repair review 的真实数据源已固定为受保护 HTTP 只读 endpoint：
  `GET /api/sc/git-mirror/repair-review`。
- Endpoint 读取 server-side `git_mirror_commits` side table，并复用 core
  `GitMirrorRepairAction` schema 生成 record-level review data。
- 返回数据包含 repo、manual-only 标记、authority note、每条 out-of-sync record 的
  action code、subject、next step、retry command 与 failure metadata。

## 边界

- Endpoint 不运行 Git，不执行 `deve_cli git export`，不写 `.git` 或 `.notegit`。
- Web 不解析 CLI 文本；CLI output 仍只是人工诊断界面。
- `.notegit` / ledger source-control state 仍是 authority，`.git` 只是 projection mirror。
- 当前只开放 review 数据，不开放 Web 后端 repair writer 或后台 executor。

## 下一步

- Web Source Control repair review 应消费该 endpoint 替换静态 review copy。
- UI 消费必须保持 read-only：只展示 record-level 数据与 copyable retry command，不调用 clipboard API，不发起 Git write。
