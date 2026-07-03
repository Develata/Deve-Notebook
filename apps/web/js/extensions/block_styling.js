import { ViewPlugin, Decoration } from "@codemirror/view";
import { syntaxTree } from "@codemirror/language";

const ATX_HEADING_LINE_RE = /^ {0,3}(#{1,6})(?:[ \t]|$)/;
// Editing affordance only: keep active CJK "#标题" candidates tall without changing saved Markdown semantics.
const ACTIVE_CJK_ATX_HEADING_LINE_RE = /^ {0,3}(#{1,6})(?=[^\s#\x00-\x7F])/u;

function atxHeadingLevel(lineText, isActiveLine) {
  const atxHeading = lineText.match(ATX_HEADING_LINE_RE);
  if (atxHeading) return atxHeading[1].length;

  const activeCjkCandidate = isActiveLine
    ? lineText.match(ACTIVE_CJK_ATX_HEADING_LINE_RE)
    : null;
  return activeCjkCandidate ? activeCjkCandidate[1].length : null;
}

function headingLineClass(headingLevel) {
  const baseClass = `cm-h${headingLevel}`;
  if (headingLevel <= 3) {
    return `${baseClass} cm-heading-line cm-heading-line-${headingLevel}`;
  }
  return baseClass;
}

/**
 * Block Styling Plugin
 * 
 * Iterates through the Markdown syntax tree and applies background classes 
 * to lines that are part of:
 * 1. Fenced Code Block (.cm-code-block-line)
 * 2. Blockquote (.cm-blockquote-line)
 * 3. ATX Headings (.cm-h1 ... .cm-h6)
 */
export const blockStyling = ViewPlugin.fromClass(
  class {
    constructor(view) {
      this.decorations = this.computeDecorations(view);
    }

    update(update) {
      if (
        update.docChanged ||
        update.viewportChanged ||
        update.searchChanged ||
        update.selectionSet
      ) {
        this.decorations = this.computeDecorations(update.view);
      }
    }

    computeDecorations(view) {
      let widgets = [];
      const codeLineStarts = new Set();
      const { from, to } = view.viewport;
      const doc = view.state.doc;
      const activeLineNumber = doc.lineAt(view.state.selection.main.head).number;

      // 遍历语法树
      const tree = syntaxTree(view.state);
      
      tree.iterate({
        from,
        to,
        enter: (node) => {
          // 处理 FencedCode, IndentedCode 和通用的 CodeBlock
          if (["FencedCode", "IndentedCode", "CodeBlock"].includes(node.name)) {
             let startLine = doc.lineAt(node.from);
             let endLine = doc.lineAt(node.to);
             
             for (let i = startLine.number; i <= endLine.number; i++) {
                 let line = doc.line(i);
                 let lineClasses = "cm-code-block-line";
                 
                 // 圆角逻辑
                 if (i === startLine.number) lineClasses += " cm-code-block-start";
                 if (i === endLine.number) lineClasses += " cm-code-block-end";
                 
                 codeLineStarts.add(line.from);
                 widgets.push(Decoration.line({ class: lineClasses }).range(line.from));
             }
          }
        },
      });

      const firstLine = doc.lineAt(from);
      const lastLine = doc.lineAt(to);
      for (let i = firstLine.number; i <= lastLine.number; i++) {
          const line = doc.line(i);
          if (codeLineStarts.has(line.from)) continue;
          const headingLevel = atxHeadingLevel(line.text, i === activeLineNumber);
          if (!headingLevel) continue;
          widgets.push(
              Decoration.line({ class: headingLineClass(headingLevel) }).range(line.from)
          );
      }

      return Decoration.set(widgets, true); 
    }
  },
  {
    decorations: (v) => v.decorations,
  }
);
