# 00_schema.md - User Operation 粒度标准

本目录定义面向架构蓝图的最小用户操作单元。目标不是罗列功能，而是精确描述“用户做了哪一步动作，这一步动作触发了哪条响应流”。

## 0. 权威关系

- `docs/plan/` 是系统蓝图的权威源。
- `docs/features/operations/` 必须与 plan 最终构成严格双射。
- 实现也必须与 plan 最终构成严格双射；若实现落后，以 plan 为准。
- operation 文档的职责是把 plan 中的能力拆成可观察、可追踪的原子用户动作，而不是用实现反推需求。

## 1. 建模单位

- 一个文件描述一组紧密相关的 user operations，例如“登录流”。
- 一个 operation 必须是用户可直接执行的单一步骤，不能再包含多个独立意图。
- `login`、`open settings`、`edit doc` 这类词通常太粗；应拆成输入、点击、提交、确认、关闭、跳转等原子动作。

## 2. 四层主骨架

每个 operation 都必须能映射到四层 canonical call architecture：

1. `User Operation`
2. `Instruction Interface`
3. `Flow Coordination`
4. `Execution Domain`

要求：

- 第一层只写用户动作，不写 CLI / Palette / Slash 这类入口类别。
- 入口类别属于 `surface` 或 `trigger` 属性，不属于层级节点名。
- 第二层写 handler、callback、command runner、form action、request sender；它负责把动作接成标准化内部指令。
- 第三层写承接该 flow 的协调模块；它负责组织步骤、推进状态、协调错误与后续任务。
- 第四层写最终收束到的 capability / authority / runtime domain。

补充：

- `Object Plane` 不是第五个主调用层，而是被多个 execution domain 共同读写的对象平面。
- `Ownership Axis` 不是主层级；它只表达模块或执行域长期归属到哪个根域。

## 3. 何时算一个合格 operation

一个 operation 必须同时满足：

- 用户可以独立执行。
- 执行完成后系统状态发生可观察变化。
- 能指出唯一主响应入口。
- 能写出至少一条稳定的 acceptance 检查。

不满足以上条件时，不应单独建模。

## 4. 文件模板

```md
# <domain>_<flow>.md

## Metadata
- Flow ID:
- Domain:
- Related Feature Chapters:
- Related Acceptance Cases:

## Operations
### op.<id>
- Name:
- Surface:
- Trigger:
- Preconditions:
- Immediate Result:
- Application Entry:

## Response Flows
### op.<id>
1. User Operation:
2. Instruction Interface:
3. Flow Coordination:
4. Execution Domains:

## Notes
- 仅记录用户可见行为与可追踪流向。
```

## 5. 命名规则

- `op.<domain>.<flow>.<verb>`
- 动词优先：`type-username`、`submit-login`、`open-palette`
- 避免 UI 文案耦合；名称要表达动作本身，而不是按钮皮肤。
- 若同一动作有多个 surface，用同一 operation，额外声明多个 trigger。

## 6. 与现有 docs 的关系

- `docs/features/*.md` 保留章节总览与用户体验描述。
- `docs/features/operations/*.md` 提供原子操作定义，作为架构图第一层与 plan-to-code 双射检查的数据源。
- `docs/acceptance-cases/*.md` 继续负责验收断言，不替代 operation 定义。
- `docs/plan/*.md` 继续负责 runtime contract，不下沉到 user-op 文案。

## 7. 双射要求

- 每个 operation flow 必须能追溯到至少一个明确的 plan chapter。
- 每个 plan 中声明的用户可见操作，最终都必须能在 operation 文档中找到唯一对应流。
- 不允许长期存在“plan 有、operation 没有”或“operation 有、plan 没有”的状态。
- 若某条 operation 尚未落地，文档应保留该 operation，并在 overview / diff 中显式标记缺口，而不是删除 blueprint。

## 8. 对象与归属

- 若一个 flow 会同时触达多个对象，应在 `Notes` 中明确主要对象，例如 `doc::content`、`pending_local_edit`、`confirmed_op`、`repo::scope`。
- 若一个 flow 同时落到多个 execution domain，应保留全部主落点，不得为了图干净而省略真实执行域。
- ownership 归属只用于表达长期责任边界，不得替代主调用层。
