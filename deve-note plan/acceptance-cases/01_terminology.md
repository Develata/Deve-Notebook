## 术语与规范性用语

```markdown
- case_id: TERM-001
  goal: 规范性用语定义齐全且可定位。
  preconditions:
    - 文件存在: deve-note plan/01_terminology.md
  steps:
    - run: rg -n "MUST / 必须|SHOULD / 应|MAY / 可选" "deve-note plan/01_terminology.md"
  assertions:
    - stdout_contains: "MUST / 必须"
    - stdout_contains: "SHOULD / 应"
    - stdout_contains: "MAY / 可选"

- case_id: TERM-002
  goal: 核心术语定义齐全。
  preconditions:
    - 文件存在: deve-note plan/01_terminology.md
  steps:
    - run: rg -n "Ledger|Snapshot|Projection|Vault|DocId|Path Mapping|Peer|Vector Clock" "deve-note plan/01_terminology.md"
  assertions:
    - stdout_contains_all:
        - "Ledger"
        - "Snapshot"
        - "Projection"
        - "Vault"
        - "DocId"
        - "Path Mapping"
        - "Peer"
        - "Vector Clock"

- case_id: TERM-003
  goal: 真值源约束表述明确。
  preconditions:
    - 文件存在: deve-note plan/01_terminology.md
  steps:
    - run: rg -n "唯一真值源|Source of Truth" "deve-note plan/01_terminology.md"
  assertions:
    - stdout_contains_any:
        - "唯一真值源"
        - "Source of Truth"
```
