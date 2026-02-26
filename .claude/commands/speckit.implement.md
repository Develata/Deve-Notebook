---
description: 按 tasks.md 中定义的全部任务执行实现计划。
---

## 用户输入

```text
$ARGUMENTS
```

在继续之前，你**必须**考虑用户输入（若不为空）。

## 大纲

1. 在仓库根目录执行 `.specify/scripts/powershell/check-prerequisites.ps1 -Json -RequireTasks -IncludeTasks`，解析 `FEATURE_DIR` 和 `AVAILABLE_DOCS`。所有路径必须为绝对路径。若参数含单引号（如 `I'm Groot`），使用 ` 'I'\''m Groot' ` 转义（或尽量改双引号）。

2. **检查 checklist 状态**（若存在 `FEATURE_DIR/checklists/`）：
   - 扫描 `checklists/` 下所有 checklist 文件。
   - 对每个 checklist 统计：
     - 总项数：匹配 `- [ ]`、`- [X]`、`- [x]` 的行
     - 已完成：匹配 `- [X]` 或 `- [x]`
     - 未完成：匹配 `- [ ]`
   - 生成状态表：

     ```text
     | Checklist | Total | Completed | Incomplete | Status |
     |-----------|-------|-----------|------------|--------|
     | ux.md     | 12    | 12        | 0          | ✓ PASS |
     | test.md   | 8     | 5         | 3          | ✗ FAIL |
     | security.md | 6   | 6         | 0          | ✓ PASS |
     ```

   - 计算总体状态：
     - **PASS**：所有 checklist 未完成项均为 0
     - **FAIL**：至少一个 checklist 有未完成项

   - **若存在未完成 checklist**：
     - 展示状态表
     - **停止**并询问：`Some checklists are incomplete. Do you want to proceed with implementation anyway? (yes/no)`
     - 等待用户回复
     - 若回复 `no`/`wait`/`stop`，终止执行
     - 若回复 `yes`/`proceed`/`continue`，进入第 3 步

   - **若全部通过**：
     - 展示全通过状态表
     - 自动进入第 3 步

3. 加载并分析实现上下文：
   - **必需**：读取 `tasks.md`（完整任务与执行计划）
   - **必需**：读取 `plan.md`（技术栈、架构、目录结构）
   - **若存在**：读取 `data-model.md`（实体与关系）
   - **若存在**：读取 `contracts/`（接口规格与测试要求）
   - **若存在**：读取 `research.md`（技术决策与约束）
   - **若存在**：读取 `quickstart.md`（集成场景）

4. **项目初始化校验**：
   - **必需**：根据实际项目形态创建/校验忽略文件。

   **检测与创建逻辑**：
   - 用以下命令判断仓库是否为 Git 仓库（若是则创建/校验 `.gitignore`）：

     ```sh
     git rev-parse --git-dir 2>/dev/null
     ```

   - 若存在 Dockerfile* 或 plan.md 提到 Docker -> 创建/校验 `.dockerignore`
   - 若存在 `.eslintrc*` -> 创建/校验 `.eslintignore`
   - 若存在 `eslint.config.*` -> 确认配置中的 `ignores` 覆盖必需模式
   - 若存在 `.prettierrc*` -> 创建/校验 `.prettierignore`
   - 若存在 `.npmrc` 或 `package.json` ->（若用于发布）创建/校验 `.npmignore`
   - 若存在 Terraform 文件（`*.tf`）-> 创建/校验 `.terraformignore`
   - 若存在 Helm chart -> 创建/校验 `.helmignore`

   **已有忽略文件**：只补充关键缺失模式。
   **缺失忽略文件**：按检测到技术栈创建完整模式集。

   **按技术栈的常见模式**（来自 plan.md）：
   - **Node.js/JavaScript/TypeScript**：`node_modules/`、`dist/`、`build/`、`*.log`、`.env*`
   - **Python**：`__pycache__/`、`*.pyc`、`.venv/`、`venv/`、`dist/`、`*.egg-info/`
   - **Java**：`target/`、`*.class`、`*.jar`、`.gradle/`、`build/`
   - **C#/.NET**：`bin/`、`obj/`、`*.user`、`*.suo`、`packages/`
   - **Go**：`*.exe`、`*.test`、`vendor/`、`*.out`
   - **Ruby**：`.bundle/`、`log/`、`tmp/`、`*.gem`、`vendor/bundle/`
   - **PHP**：`vendor/`、`*.log`、`*.cache`、`*.env`
   - **Rust**：`target/`、`debug/`、`release/`、`*.rs.bk`、`*.rlib`、`*.prof*`、`.idea/`、`*.log`、`.env*`
   - **Kotlin**：`build/`、`out/`、`.gradle/`、`.idea/`、`*.class`、`*.jar`、`*.iml`、`*.log`、`.env*`
   - **C++**：`build/`、`bin/`、`obj/`、`out/`、`*.o`、`*.so`、`*.a`、`*.exe`、`*.dll`、`.idea/`、`*.log`、`.env*`
   - **C**：`build/`、`bin/`、`obj/`、`out/`、`*.o`、`*.a`、`*.so`、`*.exe`、`Makefile`、`config.log`、`.idea/`、`*.log`、`.env*`
   - **Swift**：`.build/`、`DerivedData/`、`*.swiftpm/`、`Packages/`
   - **R**：`.Rproj.user/`、`.Rhistory`、`.RData`、`.Ruserdata`、`*.Rproj`、`packrat/`、`renv/`
   - **通用**：`.DS_Store`、`Thumbs.db`、`*.tmp`、`*.swp`、`.vscode/`、`.idea/`

   **工具专用模式**：
   - **Docker**：`node_modules/`、`.git/`、`Dockerfile*`、`.dockerignore`、`*.log*`、`.env*`、`coverage/`
   - **ESLint**：`node_modules/`、`dist/`、`build/`、`coverage/`、`*.min.js`
   - **Prettier**：`node_modules/`、`dist/`、`build/`、`coverage/`、`package-lock.json`、`yarn.lock`、`pnpm-lock.yaml`
   - **Terraform**：`.terraform/`、`*.tfstate*`、`*.tfvars`、`.terraform.lock.hcl`
   - **Kubernetes/k8s**：`*.secret.yaml`、`secrets/`、`.kube/`、`kubeconfig*`、`*.key`、`*.crt`

5. 解析 `tasks.md`，提取：
   - **任务阶段**：Setup、Tests、Core、Integration、Polish
   - **任务依赖**：串行与并行规则
   - **任务细节**：ID、描述、文件路径、并行标记 `[P]`
   - **执行流**：顺序与依赖要求

6. 按任务计划执行实现：
   - **按阶段推进**：完成当前阶段再进入下一阶段
   - **遵守依赖**：串行任务按序执行，并行任务 `[P]` 可并行
   - **遵循 TDD**：先执行测试任务，再执行对应实现任务
   - **按文件协同**：修改同一文件的任务必须串行
   - **阶段检查点**：每阶段结束进行验证

7. 实施执行规则：
   - **先做 Setup**：初始化结构、依赖、配置
   - **测试先行**：若需要，为契约/实体/集成场景先补测试
   - **核心开发**：实现模型、服务、CLI 命令、接口端点
   - **集成工作**：数据库连接、中间件、日志、外部服务
   - **收尾与验证**：单元测试、性能优化、文档更新

8. 进度跟踪与错误处理：
   - 每完成一个任务就汇报进度
   - 若非并行任务失败，立即停止
   - 对并行任务 `[P]`，继续成功项并报告失败项
   - 提供带上下文的清晰错误信息
   - 若无法继续，给出下一步建议
   - **重要**：任务完成后，务必在 `tasks.md` 中将其标记为 `[X]`

9. 完成验证：
   - 确认所有必需任务已完成
   - 检查实现结果与原始规格一致
   - 验证测试通过且覆盖满足要求
   - 确认实现符合技术计划
   - 输出最终状态与完成摘要

说明：该命令默认 `tasks.md` 已完整可执行。若任务缺失或不完整，先建议执行 `/speckit.tasks` 重新生成。
