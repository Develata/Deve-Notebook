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
          let className = "";
          
          if (node.name === "FencedCode") {
             className = "cm-code-block-line";
          } else if (node.name === "Blockquote") {
             className = "cm-blockquote-line";
          }
          
          if (className) {
             let doc = view.state.doc;
             // 找到起始行和结束行
             let startLine = doc.lineAt(node.from);
             let endLine = doc.lineAt(node.to);
             
             for (let i = startLine.number; i <= endLine.number; i++) {
                 let line = doc.line(i);
                 let lineClasses = className;
                 
                 // 圆角逻辑 (可选)
                 if (i === startLine.number) lineClasses += node.name === "FencedCode" ? " cm-code-block-start" : " cm-blockquote-start";
                 if (i === endLine.number) lineClasses += node.name === "FencedCode" ? " cm-code-block-end" : " cm-blockquote-end";
                 
                 // Decoration.line 会应用到整行元素 (div.cm-line)
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

