# 13_settings.md - 设置篇 (Settings)

本章汇总系统所有配置项，包括环境变量与 `config.toml` 配置。

## Environment Variables (环境变量)

| 变量名           | 默认值     | 说明                                    |
| :--------------- | :--------- | :-------------------------------------- |
| `DEVE_PROFILE`   | `standard` | 性能模式预设 (`standard` / `low-spec`). |
| `FEATURE_SSR`    | `true`     | 是否启用服务端渲染.                     |
| `FEATURE_SEARCH` | `true`     | 是否启用全文搜索.                       |
| `FEATURE_GRAPH`  | `true`     | 是否启用图谱分析.                       |
| `MEM_CACHE_MB`   | `128`      | 内存缓存上限 (MB).                      |
| `SYNC_MODE`      | `auto`     | 同步模式 (`auto` / `manual`).           |
| `AUTH_SECRET`    | -          | Session/JWT 签名密钥.                   |
| `AUTH_USER`      | -          | 默认用户名.                             |
| `AUTH_PASS`      | -          | 默认密码.                               |

## Configuration (config.toml)

*   **UI Settings**:
    *   `ui.recent_commands_count` (Default: 3)
    *   `ui.recent_docs_count` (Default: 10)
    *   `ui.locale`: 界面语言 (e.g., `zh-CN`).

*   **Rendering Settings**:
    *   `rendering.engine`: `KaTeX` | `MathJax`.

*   **Paths**:
    *   `vault.path`: `/data/vault`
    *   `ledger.path`: `/data/ledger`

## Rules
*   `.deveignore`: 文件忽略规则定义.
