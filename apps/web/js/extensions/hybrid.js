import { ViewPlugin, Decoration } from "@codemirror/view";
import { syntaxTree } from "@codemirror/language";
import { findMathRanges } from "./utils.js";

/**
 * Hybrid Plugin (混合插件)
 * 
 * 作用:
 * 1. 隐藏 Markdown 的部分语法标记 (如 Header 的 #, Emphasis 的 *)
 * 2. 渲染 Checkbox
 * 3. 避免在数学公式内部进行处理
 */
export const hybridPlugin = ViewPlugin.fromClass(
  class {
    constructor(view) {
      this.decorations = this.computeDecorations(view);
    }
    
    update(update) {
      if (update.docChanged || update.viewportChanged || update.selectionSet) {
        this.decorations = this.computeDecorations(update.view);
      }
    }
    
    computeDecorations(view) {
      let widgets = [];
      const { from, to } = view.viewport;
      const selection = view.state.selection.main;
      const doc = view.state.doc.toString();
      
      // 辅助函数: 检查光标是否在范围内
      const isCursorIn = (nodeFrom, nodeTo) =>
        selection.head >= nodeFrom && selection.head <= nodeTo;

      // 1. 获取所有数学公式范围，避免处理公式内的内容
      const mathRanges = findMathRanges(doc);
      
      const isInsideMath = (nodeFrom, nodeTo) => {
        for (let r of mathRanges) {
          // 只要有重叠就视为在公式内 (简单的碰撞检测)
          if (Math.max(nodeFrom, r.from) <= Math.min(nodeTo, r.to)) return true;
        }
        return false;
      };

      try {
        let tree = syntaxTree(view.state);

        tree.iterate({
          from,
          to,
          enter: (node) => {
            // DEBUG: Log node name
            // console.log("Node:", node.name, node.from, node.to);

            // 跳过数学公式区域
            if (isInsideMath(node.from, node.to)) return;

            // 隐藏标题的 # 符号 和 强调符号 * _ 和 引用符号 > 和 行内代码标记 `
            if (node.name === "HeaderMark" || node.name === "EmphasisMark" || node.name === "QuoteMark" || node.name === "CodeMark") {
              const parent = node.node.parent;
              // 只有当光标不在该行/区域时才隐藏
              if (parent && !isCursorIn(parent.from, parent.to)) {
                widgets.push(Decoration.mark({ class: "cm-syntax-hidden" }).range(node.from, node.to));
              }
            }


            // Frontmatter 样式标记
            if (node.name === "Frontmatter") {
              // 为整个 Frontmatter 块添加 CSS 类
              widgets.push(
                Decoration.mark({ class: "cm-frontmatter" }).range(
                  node.from,
                  node.to
                )
              );
            }

            // Inline Code 样式标记
            if (node.name === "InlineCode") {
                // 添加背景色样式
                widgets.push(
                    Decoration.mark({ class: "cm-inline-code" }).range(
                        node.from,
                        node.to
                    )
                );
            }
          },
        });
      } catch (e) {
        console.warn("HybridPlugin Error:", e);
      }

      return Decoration.set(widgets);
    }
  },
  { decorations: (v) => v.decorations }
);
