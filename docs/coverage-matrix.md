# Coverage Matrix

本文件维护三层文档的稳定映射：

- `docs/plan/` = engineering blueprint
- `docs/features/` = manual feature verification via Chrome MCP
- `docs/acceptance-cases/` = automation-oriented validation

## Chapter Mapping

| Blueprint | Feature Spec | Acceptance Cases | Primary Verification |
| --- | --- | --- | --- |
| `01_terminology` | `01_terminology` | `01_terminology` | glossary / lint / naming checks |
| `02_positioning` | `02_positioning` | `02_positioning` | CLI + architecture boundary checks |
| `03_rendering` | `03_rendering` | `03_rendering` | Chrome MCP + rendering automation |
| `04_storage` | `04_storage` | `07_storage_repo` | storage/runtime/repair automation |
| `05_network` | `05_network` | `06_network` | protocol + reconnect automation |
| `06_repository` | `06_repository` | `07_storage_repo` | repo/scope automation + Chrome MCP |
| `07_diff_logic` | `07_diff_logic` | `04_diff` | diff/merge/source-control automation |
| `08_ui_design` | `08_ui_design` | `05_ui`, `13_ui_mobile_chat_regression` | Chrome MCP + UI automation |
| `08_ui_design_01_web` | `08_ui_design_01_web` | `05_ui` | Web shell automation + Chrome MCP |
| `08_ui_design_02_desktop` | `08_ui_design_02_desktop` | `05_ui` | desktop shell automation |
| `08_ui_design_03_mobile` | `08_ui_design_03_mobile` | `05_ui`, `13_ui_mobile_chat_regression` | mobile shell automation + Chrome MCP |
| `09_auth` | `09_auth` | `08_auth` | auth/security automation |
| `10_ai_agent` | `10_ai_agent` | `10_plugins` | external runtime boundary automation |
| `11_i18n` | `11_i18n` | `09_i18n` | locale/error-code automation |
| `12_commands` | `12_commands` | `11_commands_settings` | command/control automation |
| `13_settings` | `13_settings` | `11_commands_settings` | settings persistence automation |
| `14_tech_stack` | `14_tech_stack` | `12_tech_release` | build/runtime budget checks |
| `15_release` | `15_release` | `12_tech_release` | packaging/CI/release automation |
| `16_web_thin_client_ledger` | `16_web_thin_client_ledger` | `06_network`, `07_storage_repo` | pending/ack/reject automation + Chrome MCP |
| `17_plugins` | `17_plugins` | `10_plugins` | plugin/runtime boundary automation |

## Rules

- Every `docs/plan/` chapter must have a corresponding row in this matrix.
- Every `docs/features/` chapter must define at least one Chrome MCP walkthrough.
- Every `docs/acceptance-cases/` file must map to at least one stable automation surface.
- A single acceptance file may cover multiple blueprint chapters, but the mapping must be explicit here.
- `08_ui_design.md` remains the shared cross-surface feature chapter.
- `08_ui_design_01/02/03` define Web / Desktop / Mobile shell behavior at the feature level and mirror the adapter split in `docs/plan/`.
