import { ViewPlugin, Decoration } from "@codemirror/view";
import { syntaxTree } from "@codemirror/language";

/**
 * Code Block Highlight Plugin
 * 
 * Iterates through the Markdown syntax tree and applies a background class 
 * to lines that are part of a Fenced Code Block.
 */
export const codeBlockHighlight = ViewPlugin.fromClass(
  class {
    constructor(view) {
      this.decorations = this.computeDecorations(view);
    }

    update(update) {
      if (update.docChanged || update.viewportChanged || update.searchChanged) {
        this.decorations = this.computeDecorations(update.view);
      }
    }

    computeDecorations(view) {
      let widgets = [];
      const { from, to } = view.viewport;

      // 遍历语法树
      const tree = syntaxTree(view.state);
      
      tree.iterate({
        from,
        to,
        enter: (node) => {
          // 在 Markdown (Lezer) 中，代码块通常是 "FencedCode"
          if (node.name === "FencedCode") {
             // 我们对每一行都应用 Line Decoration
             // 注意: node.from 和 node.to 覆盖了整个块
             // 我们需要找出块内的每一行
             
             let doc = view.state.doc;
             // 找到起始行和结束行
             let startLine = doc.lineAt(node.from);
             let endLine = doc.lineAt(node.to);
             
             for (let i = startLine.number; i <= endLine.number; i++) {
                 let line = doc.line(i);
                 
                 let classNames = "cm-code-block-line";
                 
                 // 圆角逻辑 (可选)
                 if (i === startLine.number) classNames += " cm-code-block-start";
                 if (i === endLine.number) classNames += " cm-code-block-end";
                 
                 // Decoration.line 会应用到整行元素 (div.cm-line)
                 widgets.push(Decoration.line({ class: classNames }).range(line.from));
             }
          }
        },
      });

      return Decoration.set(widgets, true); // true 表示有序添加 (实际上 Line Decoration 只要位置对就行)
    }
  },
  {
    decorations: (v) => v.decorations,
  }
);
