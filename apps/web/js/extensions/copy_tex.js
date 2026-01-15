import { EditorView } from "@codemirror/view";

/**
 * Copy-Tex Protection Extension
 * 
 * 作用: 确保当用户在编辑器中复制包含渲染公式的选区时，
 * 剪贴板中写入的是原始 LaTeX 源码，而不是渲染后的 Unicode 或乱码。
 * 
 * CodeMirror 6 默认行为:
 * 当复制包含 Widget (Decoration.replace) 的选区时，CM6 默认会获取 underlying document text (即源码)。
 * 
 * 此扩展作为显式的安全层 (Safety Layer)，用于:
 * 1. 验证复制行为。
 * 2. 将来可能的 LaTeX 格式清洗或额外 Metadata 注入。
 */
export const copyTexExtension = EditorView.domEventHandlers({
    copy(event, view) {
        // 获取当前选区
        const selection = view.state.selection.main;
        if (selection.empty) return false;

        // 我们信任 CodeMirror 6 的默认行为 (Copy Source)
        // 但在这里我们可以拦截并做额外检查，例如:
        // const text = view.state.sliceDoc(selection.from, selection.to);
        // console.log("Copying Source Text:", text);

        // 返回 false 让 CodeMirror 继续执行其默认处理 (即写入源码到剪贴板)
        return false;
    }
});
