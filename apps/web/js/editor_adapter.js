import { EditorView } from "@codemirror/view";
import { EditorState } from "@codemirror/state";
import {
  keymap,
  highlightSpecialChars,
  dropCursor,
  rectangularSelection,
  crosshairCursor,
  lineNumbers,
  highlightActiveLine,
  highlightActiveLineGutter,
} from "@codemirror/view";
import { markdown, markdownLanguage } from "@codemirror/lang-markdown";
import { GFM } from "@lezer/markdown";
import {
  defaultHighlightStyle,
  syntaxHighlighting,
  bracketMatching,
} from "@codemirror/language";
import {
  defaultKeymap,
  history,
  historyKeymap,
  redo,
  selectAll,
  undo,
} from "@codemirror/commands";

import { languages } from "@codemirror/language-data";
import { mathStateField } from "./extensions/math.js";
import { renderRangeIndexField } from "./extensions/render_range_index.js";
import { renderThemeObserver } from "./extensions/render_effects.js";
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
import { gutterDiffExtension, updateGutterDiff } from "./extensions/gutter_diff.js";
import {
  insertToolbarTaskItem,
  mobileTaskItemExtension,
} from "./extensions/mobile_task_item.js";

// --- 共享状态与远程操作 (从子模块导入) ---
import { ctx } from "./editor_state.js";
import {
  activateEditorMount,
  destroyOwnedEditorMount,
} from "./editor_lifecycle.js";
import {
  getEditorContent,
  applyRemoteContent,
  applyRemoteOp,
  applyRemoteOpsBatch,
  scrollGlobal,
  setReadOnly,
  setReadOnlyForHost,
} from "./editor_remote_ops.js";

function registerBrowserBridgeGlobal(name, value, meta = {}) {
  const target = typeof window !== "undefined" ? window : globalThis;
  const bridge = target.__deveWebBridge;
  const bridgeMeta = {
    runtime: "render_projection_runtime",
    source: "editor_adapter",
    authority: "none",
    ...meta,
  };

  if (bridge && typeof bridge.register === "function") {
    return bridge.register(name, value, bridgeMeta);
  }

  throw new Error(`web bridge registry unavailable before registering ${name}`);
}

// --- 基础设置 (Basic Setup) ---
function closeBrackets() {
  return [];
}

const manualBasicSetup = [
  lineNumbers(),
  highlightActiveLine(),
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
  let offset = 0;
  changes.iterChanges((fromA, toA, fromB, toB, inserted) => {
    deltas.push({
      from: fromA + offset,
      to: toA + offset,
      insert: inserted.toString(),
    });
    offset += (toB - fromB) - (toA - fromA);
  });
  return deltas;
}

// --- 核心初始化 (Core Initialization) ---
export function initCodeMirror(element, onDelta) {
  if (!element) return;

  try {
    return activateEditorMount(
      ctx,
      element,
      () => {
        ctx.onDeltaCallback = onDelta;
        const startState = EditorState.create({
          doc: "",
          extensions: [
            ...manualBasicSetup,
            ctx.readOnlyCompartment.of(EditorState.readOnly.of(true)),
            EditorView.lineWrapping,
            markdown({
              base: markdownLanguage,
              codeLanguages: languages,
              extensions: [GFM],
            }),
            mobileTaskItemExtension,
            renderRangeIndexField,
            renderThemeObserver,
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
            gutterDiffExtension,

            EditorView.updateListener.of((v) => {
              if (ctx.isRemote) return;
              if (v.docChanged && ctx.onDeltaCallback) {
                const deltas = changeSetToDeltas(v.changes);
                ctx.onDeltaCallback(JSON.stringify(deltas));
              }
            }),
          ],
        });
        element.innerHTML = "";
        return new EditorView({ state: startState, parent: element });
      },
    );
  } catch (e) {
    ctx.onDeltaCallback = null;
    console.error("Init Error:", e);
    throw e;
  }
}

export function destroyEditor(expectedHost) {
  const ownsMount = expectedHost
    && ctx.activeHost === expectedHost
    && ctx.activeView;
  if (!ownsMount) return false;
  try {
    destroyOwnedEditorMount(ctx, expectedHost);
    return true;
  } finally {
    ctx.onDeltaCallback = null;
  }
}

export function syncEditorStateToRust() {
  if (typeof ctx.onDeltaCallback === "function") {
    ctx.onDeltaCallback("[]");
  }
}

function activeWritableMobileView() {
  const view = ctx.activeView;
  if (!view) return false;
  if (view.state?.readOnly) return false;
  return view;
}

export function mobileInsertText(text) {
  const view = activeWritableMobileView();
  if (!view || typeof text !== "string") return false;
  const sel = view.state.selection.main;
  view.dispatch({
    changes: { from: sel.from, to: sel.to, insert: text },
    selection: { anchor: sel.from + text.length },
  });
  view.focus();
  return true;
}

export function mobileInsertTaskItem() {
  return insertToolbarTaskItem(activeWritableMobileView());
}

export function mobileWrapSelection(prefix, suffix) {
  const view = activeWritableMobileView();
  if (!view) return false;
  const sel = view.state.selection.main;
  const p = String(prefix || "");
  const s = String(suffix || "");
  const selected = view.state.sliceDoc(sel.from, sel.to);
  const insert = `${p}${selected}${s}`;
  const anchor = selected.length > 0 ? sel.from + insert.length : sel.from + p.length;
  view.dispatch({
    changes: { from: sel.from, to: sel.to, insert },
    selection: { anchor },
  });
  view.focus();
  return true;
}

function runMobileHistoryCommand(command) {
  const view = activeWritableMobileView();
  if (!view) return false;
  view.focus();
  return command(view);
}

export function mobileUndo() {
  return runMobileHistoryCommand(undo);
}

export function mobileRedo() {
  return runMobileHistoryCommand(redo);
}

// --- Re-export for window bindings ---
function getEditorSelection() {
  const view = ctx.activeView;
  if (!view) {
    return JSON.stringify(null);
  }

  const main = view.state.selection.main;
  if (main.from === main.to) {
    return JSON.stringify(null);
  }

  let text = view.state.sliceDoc(main.from, main.to);
  if (text.length > 2000) {
    text = text.slice(0, 2000);
  }

  return JSON.stringify({ from: main.from, to: main.to, text });
}

function getEditorSelectionIdentity() {
  const view = ctx.activeView;
  if (!view) return JSON.stringify(null);
  const main = view.state.selection.main;
  return JSON.stringify({
    from: main.from,
    to: main.to,
    rangeCount: view.state.selection.ranges.length,
  });
}

export { getEditorContent, applyRemoteContent, applyRemoteOp, applyRemoteOpsBatch, scrollGlobal, setReadOnly, setReadOnlyForHost, getEditorSelection, getEditorSelectionIdentity };
export { updateGutterDiff };

// --- 暴露到全局作用域供 WASM 调用 ---
registerBrowserBridgeGlobal("setupCodeMirror", initCodeMirror, { role: "wasm-editor-mount" });
registerBrowserBridgeGlobal("destroyEditor", destroyEditor, { role: "wasm-editor-lifecycle" });
registerBrowserBridgeGlobal("getEditorContent", getEditorContent, { role: "wasm-editor-query" });
registerBrowserBridgeGlobal("applyRemoteContent", applyRemoteContent, { role: "wasm-editor-snapshot" });
registerBrowserBridgeGlobal("applyRemoteOp", applyRemoteOp, { role: "wasm-editor-op" });
registerBrowserBridgeGlobal("applyRemoteOpsBatch", applyRemoteOpsBatch, { role: "wasm-editor-op-batch" });
registerBrowserBridgeGlobal("syncEditorStateToRust", syncEditorStateToRust, { role: "wasm-editor-sync" });
registerBrowserBridgeGlobal("scrollGlobal", scrollGlobal, {
  runtime: "widget_bridge_runtime",
  role: "wasm-editor-navigation",
});
registerBrowserBridgeGlobal("setReadOnly", setReadOnly, { role: "wasm-editor-readonly" });
registerBrowserBridgeGlobal("setReadOnlyForHost", setReadOnlyForHost, {
  role: "wasm-editor-owner-readonly",
});
registerBrowserBridgeGlobal("updateGutterDiff", updateGutterDiff, {
  runtime: "widget_bridge_runtime",
  role: "wasm-editor-diff-projection",
});
registerBrowserBridgeGlobal("getEditorSelection", getEditorSelection, { role: "wasm-editor-selection" });
registerBrowserBridgeGlobal("getEditorSelectionIdentity", getEditorSelectionIdentity, {
  role: "target-host-editor-selection-identity",
});
registerBrowserBridgeGlobal("mobileInsertText", mobileInsertText, {
  runtime: "widget_bridge_runtime",
  role: "wasm-editor-mobile-input",
});
registerBrowserBridgeGlobal("mobileInsertTaskItem", mobileInsertTaskItem, {
  runtime: "widget_bridge_runtime",
  role: "wasm-editor-mobile-semantic-intent",
});
registerBrowserBridgeGlobal("mobileWrapSelection", mobileWrapSelection, {
  runtime: "widget_bridge_runtime",
  role: "wasm-editor-mobile-input",
});
registerBrowserBridgeGlobal("mobileUndo", mobileUndo, {
  runtime: "widget_bridge_runtime",
  role: "wasm-editor-history",
});
registerBrowserBridgeGlobal("mobileRedo", mobileRedo, {
  runtime: "widget_bridge_runtime",
  role: "wasm-editor-history",
});
