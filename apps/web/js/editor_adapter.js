import { EditorView } from "@codemirror/view";
import { EditorState, Compartment } from "@codemirror/state";
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
import { markdown, markdownLanguage } from "@codemirror/lang-markdown";
import { GFM, Subscript, Superscript, Emoji } from "@lezer/markdown";
import {
  defaultHighlightStyle,
  syntaxHighlighting,
  bracketMatching,
} from "@codemirror/language";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";

import { languages } from "@codemirror/language-data"; // [NEW] Import languages
import { mathStateField } from "./extensions/math.js";
import { hybridPlugin } from "./extensions/hybrid.js";
import { tableStateField } from "./extensions/table.js";
import { imageStateField } from "./extensions/image.js"; 
import { checkboxStateField } from "./extensions/checkbox_ext.js"; // [NEW] Checkbox StateField
import { blockStyling } from "./extensions/block_styling.js"; // [NEW] Block Styling (Code & Quote)
import { mermaidStateField } from "./extensions/mermaid.js"; // [NEW] Mermaid Diagram

console.log("Modules Loaded via ES Imports in editor_adapter.js (v3 - ReadOnly Compartment)");
console.log("GFM Extensions:", GFM);

// --- 内部状态 (Internal State) ---
let activeView = null;
let isRemote = false;
let readOnlyCompartment = new Compartment();

// --- 基础设置 (Basic Setup) ---
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

// --- 核心初始化 (Core Initialization) ---
export function initCodeMirror(element, onUpdate) {
  console.log("Initializing Editor via Adapter (Singleton)");
  if (!element) return;
  element.innerHTML = "";

  try {
    const startState = EditorState.create({
      doc: "# Loading...",
      extensions: [
        ...manualBasicSetup,
        readOnlyCompartment.of(EditorState.readOnly.of(false)), // 默认可编辑
        EditorView.lineWrapping, 
        EditorView.lineWrapping, 
        markdown({ 
            base: markdownLanguage,
            codeLanguages: languages,
            extensions: [...GFM, Subscript, Superscript, Emoji] 
        }),
        hybridPlugin,     // 混合插件 (隐藏标记等)
        mathStateField,   // 数学公式
        tableStateField,  // 表格
        imageStateField,  // 图片
        checkboxStateField, // 复选框
        mermaidStateField,
        blockStyling, // [NEW] 代码块 & 引用块 背景高亮
        EditorView.updateListener.of((v) => {
          // 内部检查: 显式的 isRemote 标志
          if (isRemote) return;
          if (v.docChanged && onUpdate) onUpdate(v.state.doc.toString());
        }),
      ],
    });

    const view = new EditorView({
      state: startState,
      parent: element,
    });

    // 捕获单例实例
    activeView = view;
    // 暴露给调试使用 (如果绝对必要)
    window._debug_view = view; 

    return view;
  } catch (e) {
    console.error("Init Error:", e);
    throw e;
  }
}

// --- 公共 API (Public API) ---

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
    
    // 滚动到指定行的内部实现
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

export function setReadOnly(readOnly) {
    if (activeView) {
        activeView.dispatch({
            effects: readOnlyCompartment.reconfigure(EditorState.readOnly.of(readOnly))
        });
        // 强制 DOM 更新以进行视觉验证和稳健性检查
        activeView.contentDOM.setAttribute("contenteditable", (!readOnly).toString());
    }
}
