---
description: 基于可用设计制品，为当前特性生成可执行、按依赖排序的 tasks.md。
handoffs:
  - label: Analyze For Consistency
    agent: speckit.analyze
    prompt: Run a project analysis for consistency
    send: true
  - label: Implement Project
    agent: speckit.implement
    prompt: Start the implementation in phases
    send: true
---

## 用户输入

```text
$ARGUMENTS
```

在继续之前，你**必须**考虑用户输入（若不为空）。

## 大纲

1. **准备**：在仓库根目录执行 `.specify/scripts/powershell/check-prerequisites.ps1 -Json`，解析 `FEATURE_DIR` 与 `AVAILABLE_DOCS`。所有路径必须是绝对路径。若参数含单引号（如 `I'm Groot`），用 ` 'I'\''m Groot' ` 转义。

2. **加载设计文档**（位于 `FEATURE_DIR`）：
   - **必需**：`plan.md`（技术栈/库/结构）、`spec.md`（含优先级的用户故事）
   - **可选**：`data-model.md`（实体）、`contracts/`（接口契约）、`research.md`（决策）、`quickstart.md`（测试场景）
   - 说明：并非所有项目都有全部文档，应基于现有文档生成任务。

3. **执行任务生成工作流**：
   - 读取 `plan.md`，提取技术栈、库、项目结构
   - 读取 `spec.md`，提取用户故事及优先级（P1/P2/P3...）
   - 若有 `data-model.md`：提取实体并映射到用户故事
   - 若有 `contracts/`：将接口契约映射到用户故事
   - 若有 `research.md`：提取可转化为 setup 的决策
   - 按“用户故事组织”规则生成任务（见下文）
   - 生成用户故事完成顺序依赖图
   - 为每个用户故事给出并行执行示例
   - 校验任务完整性（每个故事可独立测试、任务足够完整）

4. **生成 tasks.md**：以 `.specify/templates/tasks-template.md` 为骨架，填充：
   - 来自 `plan.md` 的正确 feature 名称
   - Phase 1：Setup（项目初始化）
   - Phase 2：Foundational（所有故事共享阻塞前置）
   - Phase 3+：每个用户故事一阶段（按 `spec.md` 优先级）
   - 每阶段包含：故事目标、独立测试标准、测试任务（若要求）、实现任务
   - 最后阶段：Polish & Cross-Cutting Concerns
   - 所有任务必须严格符合清单格式（见下文）
   - 为每个任务写明清晰文件路径
   - 依赖章节要展示故事完成顺序
   - 每个故事给出并行执行示例
   - 实现策略章节（MVP 优先、增量交付）

5. **汇报**：输出 `tasks.md` 路径与摘要：
   - 任务总数
   - 每个用户故事的任务数
   - 已识别并行机会
   - 各故事独立测试标准
   - 建议 MVP 范围（通常仅 US1）
   - 格式校验结果（是否全部满足 checkbox/ID/标签/路径）

Context for task generation: $ARGUMENTS

`tasks.md` 必须“可立即执行”——每条任务都要具体到 LLM 无需额外上下文即可完成。

## 任务生成规则

**关键要求**：任务必须按用户故事组织，以支持独立实现与独立测试。

**测试任务可选**：仅当功能规格明确要求测试，或用户指定 TDD 时生成测试任务。

### 清单格式（必需）

每条任务必须严格遵循：

```text
- [ ] [TaskID] [P?] [Story?] Description with file path
```

**格式要素**：

1. **Checkbox**：固定以 `- [ ]` 开头
2. **Task ID**：按执行顺序递增（T001、T002、T003...）
3. **[P] 并行标记**：仅在任务可并行时使用（不同文件、且不依赖未完成任务）
4. **[Story] 标签**：仅用户故事阶段必须有
   - 形式：`[US1]`、`[US2]`、`[US3]`...
   - Setup 阶段：不加故事标签
   - Foundational 阶段：不加故事标签
   - 用户故事阶段：必须有故事标签
   - Polish 阶段：不加故事标签
5. **描述**：清晰动作 + 精确文件路径

**示例**：

- ✅ `- [ ] T001 Create project structure per implementation plan`
- ✅ `- [ ] T005 [P] Implement authentication middleware in src/middleware/auth.py`
- ✅ `- [ ] T012 [P] [US1] Create User model in src/models/user.py`
- ✅ `- [ ] T014 [US1] Implement UserService in src/services/user_service.py`
- ❌ `- [ ] Create User model`（缺 ID 与故事标签）
- ❌ `T001 [US1] Create model`（缺 checkbox）
- ❌ `- [ ] [US1] Create User model`（缺 Task ID）
- ❌ `- [ ] T001 [US1] Create model`（缺文件路径）

### 任务组织

1. **来自用户故事（spec.md）— 主组织维度**：
   - 每个用户故事（P1/P2/P3...）一个独立阶段
   - 将相关组件映射到对应故事：
     - 该故事所需模型
     - 该故事所需服务
     - 该故事所需接口/UI
     - 若要求测试：该故事对应测试
   - 标注故事间依赖（尽量保持故事独立）

2. **来自 contracts**：
   - 每个接口契约映射到其服务的用户故事
   - 若要求测试：在对应故事阶段中先生成契约测试任务 `[P]`，再生成实现任务

3. **来自 data-model**：
   - 每个实体映射到需要它的用户故事
   - 若实体服务多个故事：放到最早故事或 Setup 阶段
   - 实体关系放入对应故事阶段的服务层任务

4. **来自 setup/基础设施**：
   - 共享基础设施 -> Setup（Phase 1）
   - 全局阻塞前置 -> Foundational（Phase 2）
   - 故事特定初始化 -> 对应故事阶段

### 阶段结构

- **Phase 1**：Setup（项目初始化）
- **Phase 2**：Foundational（阻塞前置，必须先于用户故事完成）
- **Phase 3+**：按优先级排列的用户故事（P1/P2/P3...）
  - 每个故事内部顺序：Tests（如要求）-> Models -> Services -> Endpoints -> Integration
  - 每个阶段都应是可独立测试、可独立交付的增量
- **Final Phase**：Polish & Cross-Cutting Concerns
