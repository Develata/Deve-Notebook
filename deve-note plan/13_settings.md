# 13_settings.md - 设置篇 (Settings)

本章汇总系统所有配置项，包括环境变量、配置文件 (`settings.toml`) 以及快捷键映射。

## 1. Environment Variables (环境变量)

系统启动时支持的的环境变量列表：

| 变量名 (Key)                     | 默认值 (Default) | 说明 (Description)                                                  |
| :------------------------------- | :--------------- | :------------------------------------------------------------------ |
| **System Core**                  |                  |                                                                     |
| `DEVE_PROFILE`                   | `standard`       | 运行模式预设: `standard` (默认), `low-spec` (低配), `debug` (调试). |
| `LOG_LEVEL`                      | `info`           | 日志级别: `trace`, `debug`, `info`, `warn`, `error`.                |
| `MEM_CACHE_MB`                   | `128`            | 内存缓存上限 (MB).                                                  |
| **Network & Security**           |                  |                                                                     |
| `AUTH_SECRET`                    | *(Random)*       | Session/JWT 签名密钥. **生产环境 MUST 设置**.                       |
| `AUTH_USER`                      | `admin`          | 默认超管用户名.                                                     |
| `AUTH_PASS`                      | `password`       | 默认超管密码 (首次启动时生效).                                      |
| `AUTH_ALLOW_ANONYMOUS_LOCALHOST` | `false`          | 是否允许 Localhost / LAN 免密访问 (`true` / `false`).               |
| **Paths**                        |                  |                                                                     |
| `DEVE_DATA_DIR`                  | `~/.deve-note`   | 数据存储根目录.                                                     |

## 2. Configuration Settings (config.toml)

用户可配置的选项，通常存储在 `settings.toml` 或通过 GUI 修改。

### 2.1 UI Appearance (界面)
| Key                        | Type   | Default | Description                                         |
| :------------------------- | :----- | :------ | :-------------------------------------------------- |
| `ui.locale`                | String | `auto`  | 界面语言. 支持 `en-US`, `zh-CN`. `auto` 跟随浏览器. |
| `ui.theme`                 | String | `auto`  | 主题模式. `light`, `dark`, `auto`.                  |
| `ui.sidebar_visible`       | Bool   | `true`  | 是否显示 Primary Sidebar (左侧栏).                  |
| `ui.statusbar_visible`     | Bool   | `true`  | 是否显示 Status Bar (底部状态栏).                   |
| `ui.outline_visible`       | Bool   | `true`  | 是否显示 Outline Panel (右侧大纲).                  |
| `ui.outline_width`         | Number | `260`   | Outline 面板宽度 (Fixed, px).                       |
| `ui.recent_commands_count` | Number | `3`     | Command Palette 顶部显示的历史命令数.               |
| `ui.recent_docs_count`     | Number | `10`    | Quick Open 顶部显示的历史文件数.                    |

### 2.2 Core Logic (核心)
| Key (config.toml / Env) | Type   | Default    | Description                                            |
| :---------------------- | :----- | :--------- | :----------------------------------------------------- |
| `profile`               | String | `standard` | 运行模式: `standard` (全功能), `low-spec` (低配).      |
| `ledger_dir`            | String | `ledger`   | 账本存储目录 (Relative or Absolute).                   |
| `vault_path`            | String | `vault`    | 投影库根目录 (Relative or Absolute).                   |
| `sync_mode`             | String | `auto`     | 同步模式: `auto` (自动合并), `manual` (仅交换 Vector). |
| `snapshot_depth`        | Number | `100`      | 快照保留深度 (Versions per Repo).                      |
| `concurrency`           | Number | `4`        | 后台任务并发数 (Compression/GC).                       |

## 3. Keyboard Shortcuts (快捷键)

| 场景 (Scope)          | 快捷键 (Mac / Win)             | 命令 (Command)                          |
| :-------------------- | :----------------------------- | :-------------------------------------- |
| **Global Navigation** | `Cmd+Shift+P` / `Ctrl+Shift+P` | **Command Palette**: 呼出全局命令面板   |
|                       | `Cmd+P` / `Ctrl+P`             | **Quick Open**: 快速跳转文件            |
|                       | `Cmd+Shift+K` / `Ctrl+Shift+K` | **Switch Branch**: 切换分支             |
|                       | `Cmd+Shift+O` / `Ctrl+Shift+O` | **Toggle Outline**: 开关右侧大纲栏      |
|                       | `Cmd+B` / `Ctrl+B`             | **Toggle Sidebar**: 开关左侧侧边栏      |
|                       | `Cmd+L` / `Ctrl+L`             | **Toggle Language**: 循环切换界面语言   |
| **Version Control**   | `Cmd+S` / `Ctrl+S`             | **Save**: 保存当前文件 (触发 Diff 计算) |
|                       | `Cmd+Enter` / `Ctrl+Enter`     | **Commit**: 提交暂存区的更改            |
|                       | `Cmd+A` / `Ctrl+A`             | **Select All**: 全选当前文件            |

