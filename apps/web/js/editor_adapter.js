import { EditorView } from "@codemirror/view";
import { EditorState } from "@codemirror/state";
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
import { hyperlinkClickPlugin } from "./extensions/hyperlink_click.js"; // [NEW] Ctrl+Click 链接跳转

// --- 共享状态与远程操作 (从子模块导入) ---
import { ctx } from "./editor_state.js";
import {
  getEditorContent,
  applyRemoteContent,
  applyRemoteOp,
  applyRemoteOpsBatch,
  scrollGlobal,
  setReadOnly,
} from "./editor_remote_ops.js";

// --- 基础设置 (Basic Setup) ---
function closeBrackets() {
  return [];
}

const manualBasicSetup = [
  lineNumbers(),
  highlightActiveLineGutter(),
  highlightSpecialChars(),
  history(),
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
 */
function changeSetToDeltas(changes) {
  const deltas = [];
  changes.iterChanges((fromA, toA, fromB, toB, inserted) => {
    deltas.push({ from: fromA, to: toA, insert: inserted.toString() });
  });
  return deltas;
}

// --- 核心初始化 (Core Initialization) ---
export function initCodeMirror(element, onDelta) {
  if (!element) return;
  element.innerHTML = "";
  ctx.onDeltaCallback = onDelta;

  try {
    const startState = EditorState.create({
      doc: "# Loading...",
      extensions: [
        ...manualBasicSetup,
        ctx.readOnlyCompartment.of(EditorState.readOnly.of(false)),
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
        listMarkerPlugin,
        blockquoteBorderPlugin,
        codeToolbarPlugin,
        hyperlinkClickPlugin,

        EditorView.updateListener.of((v) => {
          if (ctx.isRemote) return;
          if (v.docChanged && ctx.onDeltaCallback) {
            const deltas = changeSetToDeltas(v.changes);
            ctx.onDeltaCallback(JSON.stringify(deltas));
          }
        }),
      ],
    });

    const view = new EditorView({ state: startState, parent: element });
    ctx.activeView = view;
    window._debug_view = view;
    return view;
  } catch (e) {
    console.error("Init Error:", e);
    throw e;
  }
}

export function destroyEditor() {
  if (ctx.activeView) {
    ctx.activeView.destroy();
    ctx.activeView = null;
    ctx.onDeltaCallback = null;
  }
}

// --- Re-export for window bindings ---
export { getEditorContent, applyRemoteContent, applyRemoteOp, applyRemoteOpsBatch, scrollGlobal, setReadOnly };

// --- 暴露到全局作用域供 WASM 调用 ---
window.setupCodeMirror = initCodeMirror;
window.destroyEditor = destroyEditor;
window.getEditorContent = getEditorContent;
window.applyRemoteContent = applyRemoteContent;
window.applyRemoteOp = applyRemoteOp;
window.applyRemoteOpsBatch = applyRemoteOpsBatch;
globalThis.applyRemoteOpsBatch = applyRemoteOpsBatch;
window.scrollGlobal = scrollGlobal;
window.setReadOnly = setReadOnly;
