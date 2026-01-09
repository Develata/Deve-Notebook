import { EditorView } from "@codemirror/view";
import { EditorState } from "@codemirror/state";
import {
  keymap,
  highlightSpecialChars,
  drawSelection,
  dropCursor,
  rectangularSelection,
  crosshairCursor,
  lineNumbers,
  highlightActiveLineGutter,
} from "@codemirror/view";
import { markdown } from "@codemirror/lang-markdown";
import {
  defaultHighlightStyle,
  syntaxHighlighting,
  bracketMatching,
} from "@codemirror/language";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";

// Import Extensions
import { mathStateField } from "./extensions/math.js";
import { hybridPlugin } from "./extensions/hybrid.js";

console.log("Modules Loaded via ES Imports in editor_adapter.js");

// --- Basic Setup ---
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

// --- Exported Init Function ---
export function initCodeMirror(element, onUpdate) {
  console.log("Initializing Editor via Adapter (Modular)");
  if (!element) return;
  element.innerHTML = "";

  try {
    const startState = EditorState.create({
      doc: "# Loading...",
      extensions: [
        ...manualBasicSetup,
        EditorView.lineWrapping, 
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
