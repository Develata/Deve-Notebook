## 技术栈、性能预算与发布

```markdown
- case_id: TECH-001
  goal: 技术栈版本匹配计划。
  preconditions:
    - Cargo.toml/package.json 可读
  steps:
    - run: rg -n "leptos|redb|argon2|ed25519" Cargo.toml
  assertions:
    - stdout_contains: "leptos"
    - stdout_contains: "redb"

- case_id: TECH-002
  goal: Markdown 导出遵循 CommonMark + GFM。
  preconditions:
    - 准备含多种语法的文档
  steps:
    - run: deve export --format markdown --doc <doc_id> --out /tmp/export.md
    - run: rg -n "==highlight==|<div>" /tmp/export.md
  assertions:
    - stdout_empty true

- case_id: PERF-001
  goal: Low-Spec 配置禁用重能力。
  preconditions:
    - DEVE_PROFILE=low-spec
  steps:
    - run: deve config print
  assertions:
    - stdout_contains: "profile = 'low-spec'"

- case_id: REL-001
  goal: Release Channels 命名规范。
  preconditions:
    - 构建 stable release
  steps:
    - run: ls dist
  assertions:
    - stdout_contains: "v1.0.0"

- case_id: REL-002
  goal: Docker 部署可用。
  preconditions:
    - Docker 可用
    - AUTH_SECRET 已设置为 32 字节以上随机字符串
    - AUTH_PASS 已设置为 Argon2 PHC 密码哈希
  steps:
    - run: docker run -d --name deve-server -p 3001:3001 -v $(pwd)/data:/data -e DEVE_LEDGER_DIR=/data/ledger -e DEVE_VAULT_PATH=/data/vault -e AUTH_SECRET -e AUTH_PASS ghcr.io/develata/deve-notebook:latest
    - run: curl -I http://127.0.0.1:3001/api/node/role
  assertions:
    - http_status_eq: 200

- case_id: REL-003
  goal: 发布前检查项可验证。
  preconditions:
    - CI 环境
  steps:
    - run: cargo test --locked
    - run: cargo audit
  assertions:
    - exit_code_all_eq: 0
```
