import { ViewPlugin, Decoration } from "@codemirror/view";
import { syntaxTree } from "@codemirror/language";

/**
 * Block Styling Plugin
 * 
 * Iterates through the Markdown syntax tree and applies background classes 
 * to lines that are part of:
 * 1. Fenced Code Block (.cm-code-block-line)
 * 2. Blockquote (.cm-blockquote-line)
 */
export const blockStyling = ViewPlugin.fromClass(
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
          // 只处理 FencedCode (Blockquote 由 blockquote_border.js 处理)
          if (node.name === "FencedCode") {
             let doc = view.state.doc;
             let startLine = doc.lineAt(node.from);
             let endLine = doc.lineAt(node.to);
             
             for (let i = startLine.number; i <= endLine.number; i++) {
                 let line = doc.line(i);
                 let lineClasses = "cm-code-block-line";
                 
                 // 圆角逻辑
                 if (i === startLine.number) lineClasses += " cm-code-block-start";
                 if (i === endLine.number) lineClasses += " cm-code-block-end";
                 
                 widgets.push(Decoration.line({ class: lineClasses }).range(line.from));
             }
          }
        },
      });

      return Decoration.set(widgets, true); 
    }
  },
  {
    decorations: (v) => v.decorations,
  }
);

