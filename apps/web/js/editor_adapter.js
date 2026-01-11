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

import { mathStateField } from "./extensions/math.js";
import { hybridPlugin } from "./extensions/hybrid.js";
import { tableStateField } from "./extensions/table.js";

console.log("Modules Loaded via ES Imports in editor_adapter.js (v2 - Adapter Pattern)");

// --- Internal State ---
let activeView = null;
let isRemote = false;

// --- Basic Setup ---
function closeBrackets() {
  return []; 
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

// --- Core Initialization ---
export function initCodeMirror(element, onUpdate) {
  console.log("Initializing Editor via Adapter (Singleton)");
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
        tableStateField,
        EditorView.updateListener.of((v) => {
          // Internal Check: explicit isRemote flag
          if (isRemote) return;
          if (v.docChanged && onUpdate) onUpdate(v.state.doc.toString());
        }),
      ],
    });

    const view = new EditorView({
      state: startState,
      parent: element,
    });

    // Capture Singleton Instance
    activeView = view;
    // Expose for debugging if absolutely needed, but code should rely on exports
    window._debug_view = view; 

    return view;
  } catch (e) {
    console.error("Init Error:", e);
    throw e;
  }
}

// --- Public API ---

export function getEditorContent() {
  return activeView ? activeView.state.doc.toString() : "";
}

export function applyRemoteContent(text) {
  if (activeView) {
    isRemote = true;
    try {
      activeView.dispatch({
        changes: {
          from: 0,
          to: activeView.state.doc.length,
          insert: text,
        },
      });
    } catch (e) {
      console.error("applyRemoteContent Error:", e);
    } finally {
      isRemote = false;
    }
  }
}

export function applyRemoteOp(op_json) {
  if (activeView) {
    isRemote = true;
    try {
      const op = JSON.parse(op_json);
      if (op.Insert) {
        activeView.dispatch({
          changes: { from: op.Insert.pos, insert: op.Insert.content },
        });
      } else if (op.Delete) {
        activeView.dispatch({
          changes: {
            from: op.Delete.pos,
            to: op.Delete.pos + op.Delete.len,
            insert: "",
          },
        });
      }
    } catch (e) {
      console.error("applyRemoteOp Error:", e);
    } finally {
      isRemote = false;
    }
  }
}

export function scrollGlobal(lineNumber) {
    if (!activeView || !activeView.state) return;
    
    // Internal impl of scrollToLine
    const doc = activeView.state.doc;
    const lines = doc.lines;
    if (lineNumber < 1) lineNumber = 1;
    if (lineNumber > lines) lineNumber = lines;

    const line = doc.line(lineNumber);
    
    activeView.dispatch({
        effects: [
            EditorView.scrollIntoView(line.from, { y: "start", yMargin: 20 })
        ],
        selection: { anchor: line.from }
    });
    activeView.focus();
}
