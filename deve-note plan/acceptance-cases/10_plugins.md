## 插件与运行时

```markdown
- case_id: PLUG-001
  goal: Rhai 与 WASM 插件可加载。
  preconditions:
    - 已安装 demo.rhai 与 demo.wasm 插件
  steps:
    - run: deve plugin install demo.rhai
    - run: deve plugin install demo.wasm
    - run: deve plugin list
  assertions:
    - stdout_contains: "demo.rhai"
    - stdout_contains: "demo.wasm"

- case_id: PLUG-002
  goal: Capabilities 默认拒绝。
  preconditions:
    - 插件未声明 allow_net
  steps:
    - run: deve plugin call demo.rhai net_test
  assertions:
    - stdout_contains: "capability denied"

- case_id: PLUG-003
  goal: WASM 不直接操作 DOM。
  preconditions:
    - wasm 插件尝试 DOM 操作
  steps:
    - run: deve plugin call demo.wasm dom_test
  assertions:
    - stdout_contains: "dom access denied"

- case_id: PLUG-004
  goal: Podman Rootless/No Net/Ephemeral。
  preconditions:
    - Podman 可用
  steps:
    - run: deve exec run python "print('ok')"
  assertions:
    - log_contains: "rootless"
    - log_contains: "network disabled"
    - log_contains: "container removed"

- case_id: PLUG-005
  goal: AI 插件上下文安全。
  preconditions:
    - AI 插件请求上下文
  steps:
    - run: deve plugin call ai.get_context
  assertions:
    - stdout_contains_any: ["permission required", "context denied"]

- case_id: PLUG-006
  goal: KaTeX 扩展按配置加载。
  preconditions:
    - config.tex_extensions 为空
  steps:
    - ui_type: "\\ce{H2O}"
    - ui_wait_render: true
    - config_set: "config.tex_extensions" = ["mhchem"]
    - ui_reload: true
  assertions:
    - ui_assert: chemistry_not_rendered_before true
    - ui_assert: chemistry_rendered_after true
```
