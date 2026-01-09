import { WidgetType, Decoration, EditorView } from "@codemirror/view";
import { StateField } from "@codemirror/state";
import { findMathRanges } from "./utils.js";

// --- Math Widget ---
export class MathWidget extends WidgetType {
  constructor(content, isBlock) {
    super();
    this.content = content;
    this.isBlock = isBlock;
  }
  toDOM(view) {
    let span = document.createElement("span");
    span.className = "cm-math-widget" + (this.isBlock ? " cm-block-math" : "");
    try {
      if (window.katex) {
        window.katex.render(this.content, span, {
          throwOnError: false,
          displayMode: this.isBlock,
        });
      } else {
        span.innerText =
          (this.isBlock ? "$$" : "$") +
          this.content +
          (this.isBlock ? "$$" : "$"); // Fallback
      }
    } catch (e) {
      span.innerText = "Error";
    }
    return span;
  }
}

function computeMathDecorations(state) {
  let widgets = [];
  let doc = state.doc.toString();
  let selection = state.selection.main;
  
  // Use the shared parser
  const ranges = findMathRanges(doc);

  for (let r of ranges) {
      const isBlock = r.type === "BLOCK";
      const content = doc.slice(r.contentFrom, r.contentTo);
      
      const isCursorTouching = selection.head >= r.from && selection.head <= r.to;
      
      if (!isCursorTouching) {
        widgets.push(
            Decoration.replace({ 
                widget: new MathWidget(content, isBlock) 
            }).range(r.from, r.to)
        );
      }
      
      // Handle escaped dollars - parser ignores them, but we might want to hide backslashes?
      // The original code hid backslashes of escaped dollars when not touching cursor.
      // But `findMathRanges` skips escaped ones. 
      // If we want to hide `\$`, we need to find them separately or make parser return them.
      // ORIGINAL LOGIC: "If isEscaped... widgets.push(Decoration.replace({}).range(escapeStart, index))"
      // We should probably port that logic here or update utils to generic token parser.
      // For simplicity and strict refactor, let's keep the escape hiding logic LOCAL here or add it to utils?
      // Since Utils specifically finds RANGES, maybe we handle escapes separately here.
  }
  
  // Restore Escape Hiding Logic (Simplified local re-scan or we should have asked utils)
  // Let's do a quick pass for escapes as originally intended.
  const regexAnyDollar = /\$+/g;
  let match;
  while ((match = regexAnyDollar.exec(doc)) !== null) {
      const index = match.index;
      let backslashes = 0;
      let i = index - 1;
      while (i >= 0 && doc[i] === "\\") { backslashes++; i--; }
      
      if (backslashes % 2 === 1) {
          // It is escaped
          const escapeStart = index - 1;
          const end = index + match[0].length;
           const isCursorTouching = selection.head >= escapeStart && selection.head <= end;
           if (!isCursorTouching) {
             widgets.push(Decoration.replace({}).range(escapeStart, index));
           }
      }
  }

  widgets.sort((a, b) => a.from - b.from);
  return Decoration.set(widgets);
}

export const mathStateField = StateField.define({
  create(state) {
    return computeMathDecorations(state);
  },
  update(decorations, transaction) {
    if (transaction.docChanged || transaction.selection) {
      return computeMathDecorations(transaction.state);
    }
    return decorations;
  },
  provide: (f) => EditorView.decorations.from(f),
});
