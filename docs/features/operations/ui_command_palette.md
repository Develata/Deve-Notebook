# ui_command_palette.md - Command Palette 操作流示例

## Metadata

- `Flow ID`: `flow.ui.command-palette`
- `Domain`: `ui-shell`
- `Related Feature Chapters`: `docs/features/12_commands.md`, `docs/features/08_ui_design_02_desktop.md`
- `Related Acceptance Cases`: `UI-GEN-002`, `UI-GEN-003`, `CMD-002`

## Operations

### `op.ui.command-palette.open`

- `Name`: `Open Command Palette`
- `Surface`: `keyboard-shortcut`
- `Trigger`: 当前代码以 `Ctrl/Cmd+K` 为主；文档仍写 `Cmd/Ctrl+Shift+P`
- `Preconditions`: 应用主界面已加载
- `Immediate Result`: command palette overlay 显示，查询文本重置，选中项重置为首项
- `Application Entry`: `apps/web/src/components/command_palette/mod.rs`, `apps/web/src/components/command_palette/logic.rs`, `apps/web/src/components/command_palette/ui.rs`

### `op.ui.command-palette.type-query`

- `Name`: `Type Command Query`
- `Surface`: `overlay-input`
- `Trigger`: `input[name="command-palette-query"]`
- `Preconditions`: `op.ui.command-palette.open` 已执行
- `Immediate Result`: 查询字符串更新，命令列表重新过滤
- `Application Entry`: `apps/web/src/components/command_palette/ui.rs`, `apps/web/src/components/command_palette/logic.rs`, `apps/web/src/components/command_palette/registry.rs`

### `op.ui.command-palette.navigate`

- `Name`: `Navigate Command Results`
- `Surface`: `keyboard`
- `Trigger`: `ArrowUp` / `ArrowDown`
- `Preconditions`: palette 已打开，存在可选命令
- `Immediate Result`: 当前选中项变化
- `Application Entry`: `apps/web/src/components/command_palette/logic.rs`

### `op.ui.command-palette.execute`

- `Name`: `Execute Selected Command`
- `Surface`: `keyboard-or-pointer`
- `Trigger`: `Enter` 或点击某个命令项
- `Preconditions`: palette 已打开，存在选中项
- `Immediate Result`: 调用对应命令 action，随后关闭 palette 或转入下一个 UI 流
- `Application Entry`: `apps/web/src/components/command_palette/logic.rs`, `apps/web/src/components/command_palette/registry.rs`, `apps/web/src/components/command_palette/ui.rs`

### `op.ui.command-palette.close`

- `Name`: `Close Command Palette`
- `Surface`: `keyboard-or-overlay`
- `Trigger`: `Escape`、点击遮罩、再次触发关闭快捷键
- `Preconditions`: palette 已打开
- `Immediate Result`: overlay 隐藏
- `Application Entry`: `apps/web/src/components/command_palette/logic.rs`, `apps/web/src/components/command_palette/ui.rs`

## Response Flows

### `op.ui.command-palette.open`

1. `User Operation`: 用户按下打开命令面板的快捷键。
2. `Application Response`: 显示 `CommandPalette`，并通过 reset effect 清空 query、把选中索引重置为 `0`。
3. `Concrete Modules`:
   - `apps/web/src/components/command_palette/mod.rs`
   - `apps/web/src/components/command_palette/logic.rs`
   - `apps/web/src/components/command_palette/ui.rs`
   - `apps/web/src/shortcuts/global.rs`
4. `Core Subsystems`: 无。此步属于 UI shell 局部状态切换。

### `op.ui.command-palette.type-query`

1. `User Operation`: 用户在 palette 输入框输入检索词。
2. `Application Response`: 更新 `query`，重置选中索引，并重新构建过滤后的命令列表。
3. `Concrete Modules`:
   - `apps/web/src/components/command_palette/ui.rs`
   - `apps/web/src/components/command_palette/logic.rs`
   - `apps/web/src/components/command_palette/registry.rs`
4. `Core Subsystems`: 无。此步仍停留在前端命令检索层。

### `op.ui.command-palette.navigate`

1. `User Operation`: 用户按 `ArrowUp` 或 `ArrowDown` 移动选中项。
2. `Application Response`: keydown handler 读取结果数并循环更新 `selected_index`。
3. `Concrete Modules`:
   - `apps/web/src/components/command_palette/logic.rs`
4. `Core Subsystems`: 无。此步不触发业务模块。

### `op.ui.command-palette.execute`

1. `User Operation`: 用户按 Enter 或点击某条命令。
2. `Application Response`: 取当前选中 command，执行其 `action`；根据命令不同，可能打开设置、打开文档搜索、切换语言、切换 peer、触发 merge、切换 AI chat。
3. `Concrete Modules`:
   - `apps/web/src/components/command_palette/logic.rs`
   - `apps/web/src/components/command_palette/registry.rs`
   - `apps/web/src/components/search_box/`
   - `apps/web/src/components/settings.rs`
   - `apps/web/src/components/chat/`
4. `Core Subsystems`:
   - `plugin`
   - `protocol`
   - `source_control`
   - 或无。取决于被执行的具体命令。

### `op.ui.command-palette.close`

1. `User Operation`: 用户按 Escape、点击遮罩或再次触发关闭快捷键。
2. `Application Response`: 将 `show` 设为 `false`，overlay 消失。
3. `Concrete Modules`:
   - `apps/web/src/components/command_palette/logic.rs`
   - `apps/web/src/components/command_palette/ui.rs`
4. `Core Subsystems`: 无。此步只影响 UI shell。

## Notes

- 第一层应该是 `open`、`type-query`、`navigate`、`execute`、`close`，而不是“Command Palette”本身。
- `Command Palette` 是一个 UI 容器，不是 user operation。
- `execute` 是分叉点；真正进入核心模块的不是 palette，而是被选中的具体命令。
- 共享的 provider 选择与 `SearchAction` 路由已单独建模在 `command_surface_mode_routing.md` 与 `command_surface_action_routing.md`。
- 当前代码与文档对打开快捷键存在差异，这应由 overview diff 单独标记，不应影响 operation 粒度定义。
