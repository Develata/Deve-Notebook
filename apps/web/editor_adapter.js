// apps/web/public/js/editor_adapter.js

import { EditorView } from "@codemirror/view";
import { EditorState, StateField } from "@codemirror/state";
import {
  keymap,
  highlightSpecialChars,
  drawSelection,
  dropCursor,
  rectangularSelection,
  crosshairCursor,
  lineNumbers,
  highlightActiveLineGutter,
  Decoration,
  ViewPlugin,
  WidgetType,
} from "@codemirror/view";
import { markdown } from "@codemirror/lang-markdown";
import {
  syntaxTree,
  defaultHighlightStyle,
  syntaxHighlighting,
  bracketMatching,
} from "@codemirror/language";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";

console.log("Modules Loaded via ImportMap in editor_adapter.js");

// --- 1. Basic Setup ---
function closeBrackets() {
  return []; // Placeholder
}

const manualBasicSetup = [
  lineNumbers(),
  highlightActiveLineGutter(),
  highlightSpecialChars(),
  history(),
  drawSelection(),
  dropCursor(),
  EditorState.allowMultipleSelections.of(true),
  syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
  bracketMatching(),
  closeBrackets(),
  rectangularSelection(),
  crosshairCursor(),
  keymap.of([...defaultKeymap, ...historyKeymap]),
];

// --- 2. Math Widget & Plugin ---
class MathWidget extends WidgetType {
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

  const regexAnyDollar = /\$+/g;

  const isEscaped = (index) => {
    let backslashes = 0;
    let i = index - 1;
    while (i >= 0 && doc[i] === "\\") {
      backslashes++;
      i--;
    }
    return backslashes % 2 === 1;
  };

  let match;
  let tokens = [];

  while ((match = regexAnyDollar.exec(doc)) !== null) {
    const val = match[0];
    const index = match.index;

    if (isEscaped(index)) {
      const end = index + val.length;
      const escapeStart = index - 1;
      if (escapeStart >= 0) {
        const isCursorTouching =
          selection.head >= escapeStart && selection.head <= end;
        if (!isCursorTouching) {
          widgets.push(Decoration.replace({}).range(escapeStart, index));
        }
      }
      continue;
    }

    if (val === "$$" || val === "$") {
      tokens.push({ type: val, index });
    }
  }

  let mode = "NONE";
  let startToken = null;

  for (let i = 0; i < tokens.length; i++) {
    let t = tokens[i];

    if (mode === "NONE") {
      if (t.type === "$$") {
        mode = "BLOCK";
        startToken = t;
      } else if (t.type === "$") {
        mode = "INLINE";
        startToken = t;
      }
    } else if (mode === "BLOCK") {
      if (t.type === "$$") {
        let from = startToken.index;
        let to = t.index + 2;
        let content = doc.slice(from + 2, t.index);

        const isCursorIn = selection.head >= from && selection.head <= to;
        if (!isCursorIn) {
          widgets.push(
            Decoration.replace({ widget: new MathWidget(content, true) }).range(
              from,
              to
            )
          );
        }
        mode = "NONE";
        startToken = null;
      }
    } else if (mode === "INLINE") {
      let contentSoFar = doc.slice(startToken.index + 1, t.index);
      if (contentSoFar.includes("\n\n")) {
        mode = "NONE";
        startToken = null;
        i--;
        continue;
      }

      if (t.type === "$") {
        let from = startToken.index;
        let to = t.index + 1;
        let content = contentSoFar;

        const isCursorIn = selection.head >= from && selection.head <= to;
        if (!isCursorIn) {
          widgets.push(
            Decoration.replace({
              widget: new MathWidget(content, false),
            }).range(from, to)
          );
        }
        mode = "NONE";
        startToken = null;
      } else if (t.type === "$$") {
        mode = "NONE";
        startToken = null;
        i--;
        continue;
      }
    }
  }

  widgets.sort((a, b) => a.from - b.from);
  return Decoration.set(widgets);
}

const mathStateField = StateField.define({
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

// --- 3. Checkbox Widget ---
class CheckboxWidget extends WidgetType {
  constructor(checked, pos) {
    super();
    this.checked = checked;
    this.pos = pos;
  }
  toDOM(view) {
    let wrap = document.createElement("span");
    wrap.className = "cm-checkbox-widget";
    let input = document.createElement("input");
    input.type = "checkbox";
    input.checked = this.checked;
    input.className = "cursor-pointer";

    input.onclick = (e) => {
      e.preventDefault();
      const valid = this.checked ? " " : "x";
      view.dispatch({
        changes: { from: this.pos + 1, to: this.pos + 2, insert: valid },
      });
    };
    wrap.appendChild(input);
    return wrap;
  }

  ignoreEvent() {
    return false;
  }
}

// --- 4. Hybrid Plugin ---
const hybridPlugin = ViewPlugin.fromClass(
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
      const isCursorIn = (nodeFrom, nodeTo) =>
        selection.head >= nodeFrom && selection.head <= nodeTo;

      // Compute Math Ranges to Ignore (simplified for this context, same logic as above mostly)
      // For efficiency, we might want to share the parsing logic or just re-run it quickly.
      // Copying logic for robustness here.
      const mathRanges = [];
      const regexAnyDollar = /\$+/g;
      let match;
      const isEscaped = (index) => {
        let backslashes = 0;
        let i = index - 1;
        while (i >= 0 && doc[i] === "\\") {
          backslashes++;
          i--;
        }
        return backslashes % 2 === 1;
      };

      let tokens = [];
      while ((match = regexAnyDollar.exec(doc)) !== null) {
        const val = match[0];
        const index = match.index;
        if (isEscaped(index)) continue;
        if (val === "$$" || val === "$") {
          tokens.push({ type: val, index });
        }
      }

      let mode = "NONE";
      let startToken = null;
      for (let t of tokens) {
        if (mode === "NONE") {
          if (t.type === "$$") {
            mode = "BLOCK";
            startToken = t;
          } else if (t.type === "$") {
            mode = "INLINE";
            startToken = t;
          }
        } else if (mode === "BLOCK") {
          if (t.type === "$$") {
            mathRanges.push({ from: startToken.index, to: t.index + 2 });
            mode = "NONE";
            startToken = null;
          }
        } else if (mode === "INLINE") {
          if (t.type === "$") {
            mathRanges.push({ from: startToken.index, to: t.index + 1 });
            mode = "NONE";
            startToken = null;
          }
        }
      }

      const isInsideMath = (nodeFrom, nodeTo) => {
        for (let r of mathRanges) {
          if (nodeFrom >= r.from && nodeTo <= r.to) return true;
        }
        return false;
      };

      try {
        let tree = syntaxTree(view.state);

        tree.iterate({
          from,
          to,
          enter: (node) => {
            if (isInsideMath(node.from, node.to)) return;

            // Hide Header/Emphasis Marks
            if (node.name === "HeaderMark" || node.name === "EmphasisMark") {
              const parent = node.node.parent;
              if (parent && !isCursorIn(parent.from, parent.to)) {
                widgets.push(Decoration.replace({}).range(node.from, node.to));
              }
            }

            // Checkboxes
            if (node.name === "TaskMarker") {
              const slice = view.state.sliceDoc(node.from, node.to);
              const isChecked = slice.toLowerCase().includes("x");

              if (!isCursorIn(node.from, node.to)) {
                widgets.push(
                  Decoration.replace({
                    widget: new CheckboxWidget(isChecked, node.from),
                  }).range(node.from, node.to)
                );
              }
            }

            // Frontmatter
            if (node.name === "Frontmatter") {
              widgets.push(
                Decoration.mark({ class: "cm-frontmatter" }).range(
                  node.from,
                  node.to
                )
              );
            }
          },
        });
      } catch (e) {
        console.warn(e);
      }

      return Decoration.set(widgets);
    }
  },
  { decorations: (v) => v.decorations }
);

// --- 5. Exported Init Function ---
export function initCodeMirror(element, onUpdate) {
  console.log("Initializing Editor via Adapter");
  if (!element) return;
  element.innerHTML = "";

  try {
    const startState = EditorState.create({
      doc: "# Loading...",
      extensions: [
        ...manualBasicSetup,
        markdown(),
        hybridPlugin,
        mathStateField,
        EditorView.updateListener.of((v) => {
          if (window._isRemote) return;
          if (v.docChanged && onUpdate) onUpdate(v.state.doc.toString());
        }),
      ],
    });

    const view = new EditorView({
      state: startState,
      parent: element,
    });

    return view;
  } catch (e) {
    console.error("Init Error:", e);
    throw e;
  }
}

export function scrollToLine(view, lineNumber) {
    if (!view || !view.state) return;
    // Ensure line number is within bounds
    const doc = view.state.doc;
    const lines = doc.lines;
    if (lineNumber < 1) lineNumber = 1;
    if (lineNumber > lines) lineNumber = lines;

    const line = doc.line(lineNumber);
    
    view.dispatch({
        effects: [
            EditorView.scrollIntoView(line.from, { y: "start", yMargin: 20 })
        ],
        selection: { anchor: line.from }
    });
    view.focus();
}
