# 08_ui_design_03_mobile - Mobile UI

## 移动端适配策略
*   **Design Philosophy (设计哲学)**: **Content-First**。参考 **VitePress** 或 **Vue** 文档的移动端风格，追求极致的清爽与阅读体验。
*   **Navigation Strategy (导航策略)**:
    *   **Sidebar (Left)**: 默认隐藏。通过左上角 **Hamburger Menu** (`≡`) 唤出，以 **Drawer (抽屉)** 形式从左侧滑入。
    *   **Outline (Right)**: 默认隐藏。通过右上角或内容顶部的 **TOC Icon** 唤出，以 **Bottom Sheet** 或 **Drawer** 形式展示。
*   **Layout (布局)**:
    *   **Single Column**: 强制 **单列显示**，移除所有多列网格。Editor 占据 100% 宽度。
    *   **Status Bar**: 简化信息，仅保留 Sync 状态与 Read-Only 指示器。
*   **Diff View (差异对比)**:
    *   **Vertical Stack**: 移动端 **MUST NOT** 使用 Side-by-Side 对比。
    *   **Behavior**: 采用 **Unified View** 或 **Vertical Split** (Old Top / New Bottom) 展示差异。
*   **Spectator Indicator**:
    *   在 Spectator Mode 下，顶部导航栏下方 **MUST** 显示一条醒目的橙色 Banner: `"Read-Only"`，明确提示当前不可编辑。
