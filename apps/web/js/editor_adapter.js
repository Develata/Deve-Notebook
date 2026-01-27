import { EditorView } from "@codemirror/view";
import { EditorState, Compartment } from "@codemirror/state";
import {
  keymap,
  highlightSpecialChars,
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
import { defaultKeymap, history, historyKeymap, selectAll } from "@codemirror/commands";

import { languages } from "@codemirror/language-data";
import { mathStateField } from "./extensions/math.js";
import { hybridPlugin } from "./extensions/hybrid.js";
import { tableStateField } from "./extensions/table.js";
import { imageStateField } from "./extensions/image.js";
import { checkboxStateField } from "./extensions/checkbox_ext.js";
import { blockStyling } from "./extensions/block_styling.js";
import { mermaidStateField } from "./extensions/mermaid.js";
import { copyTexExtension } from "./extensions/copy_tex.js";
import { listMarkerPlugin } from "./extensions/list_marker.js";
import { blockquoteBorderPlugin } from "./extensions/blockquote_border.js";
import { codeToolbarPlugin } from "./extensions/code_toolbar.js"; // [NEW]

console.log("Editor Adapter v5 - Native Selection Mode");


// --- 内部状态 (Internal State) ---
let activeView = null;
let isRemote = false;
let readOnlyCompartment = new Compartment();

// --- 回调函数 (Callbacks) ---
/** @type {((changes: {from: number, to: number, insert: string}[]) => void) | null} */
let onDeltaCallback = null;

// --- 基础设置 (Basic Setup) ---
// 注意: 移除了 drawSelection()，使用浏览器原生选择来避免滚动时选择背景消失的问题
function closeBrackets() {
  return [];
}

const manualBasicSetup = [
  lineNumbers(),
  highlightActiveLineGutter(),
  highlightSpecialChars(),
  history(),
  // drawSelection() 已移除 - 使用原生浏览器选择以避免滚动同步问题
  dropCursor(),
  EditorState.allowMultipleSelections.of(true),
  syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
  bracketMatching(),
  closeBrackets(),
  rectangularSelection(),
  crosshairCursor(),
  keymap.of([...defaultKeymap, ...historyKeymap, { key: "Ctrl-a", run: selectAll }]),
];

/**
 * 将 CodeMirror ChangeSet 转换为 Delta 数组
 * @param {import("@codemirror/state").ChangeSet} changes
 * @returns {{from: number, to: number, insert: string}[]}
 */
function changeSetToDeltas(changes) {
  const deltas = [];
  changes.iterChanges((fromA, toA, fromB, toB, inserted) => {
    deltas.push({
      from: fromA,
      to: toA,
      insert: inserted.toString(),
    });
  });
  return deltas;
}

// --- 核心初始化 (Core Initialization) ---
/**
 * 初始化 CodeMirror 编辑器
 * @param {HTMLElement} element - 容器元素
 * @param {(changes: {from: number, to: number, insert: string}[]) => void} onDelta - Delta 回调 (性能优化)
 */
export function initCodeMirror(element, onDelta) {
  console.log("Initializing Editor (Delta Mode)");
  if (!element) return;
  element.innerHTML = "";

  // 保存回调
  onDeltaCallback = onDelta;

  try {
    const startState = EditorState.create({
      doc: "# Loading...",
      extensions: [
        ...manualBasicSetup,
        readOnlyCompartment.of(EditorState.readOnly.of(false)),
        EditorView.lineWrapping,
        markdown({
          base: markdownLanguage,
          codeLanguages: languages,
          extensions: [...GFM, Subscript, Superscript, Emoji],
        }),
        hybridPlugin,
        mathStateField,
        tableStateField,
        imageStateField,
        checkboxStateField,
        mermaidStateField,
        copyTexExtension,
        blockStyling,
        listMarkerPlugin,        // [NEW] 列表圆点渲染
        blockquoteBorderPlugin,  // [NEW] 引用块边框
        codeToolbarPlugin,       // [NEW] 代码块工具栏 (Copy/Lang)

        // 性能优化: 发送 Delta 而不是全文

        EditorView.updateListener.of((v) => {
          if (isRemote) return;
          if (v.docChanged && onDeltaCallback) {
            const deltas = changeSetToDeltas(v.changes);
            onDeltaCallback(JSON.stringify(deltas));
          }
        }),
      ],
    });

    const view = new EditorView({
      state: startState,
      parent: element,
    });

    activeView = view;
    window._debug_view = view;

    return view;
  } catch (e) {
    console.error("Init Error:", e);
    throw e;
  }
}

// --- 清理函数 (Cleanup) ---
/**
 * 销毁编辑器实例，释放资源
 */
export function destroyEditor() {
  if (activeView) {
    activeView.destroy();
    activeView = null;
    onDeltaCallback = null;
    console.log("Editor destroyed");
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

  const doc = activeView.state.doc;
  const lines = doc.lines;
  if (lineNumber < 1) lineNumber = 1;
  if (lineNumber > lines) lineNumber = lines;

  const line = doc.line(lineNumber);

  activeView.dispatch({
    effects: [
      EditorView.scrollIntoView(line.from, { y: "start", yMargin: 20 }),
    ],
    selection: { anchor: line.from },
  });
  activeView.focus();
}

export function setReadOnly(readOnly) {
  if (activeView) {
    activeView.dispatch({
      effects: readOnlyCompartment.reconfigure(
        EditorState.readOnly.of(readOnly),
      ),
    });
    activeView.contentDOM.setAttribute(
      "contenteditable",
      (!readOnly).toString(),
    );
  }
}

// --- 暴露到全局作用域供 WASM 调用 ---
window.setupCodeMirror = initCodeMirror;
window.destroyEditor = destroyEditor;
window.getEditorContent = getEditorContent;
window.applyRemoteContent = applyRemoteContent;
window.applyRemoteOp = applyRemoteOp;
window.scrollGlobal = scrollGlobal;
window.setReadOnly = setReadOnly;
