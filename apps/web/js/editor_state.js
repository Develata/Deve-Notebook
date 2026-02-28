// js/editor_state.js
// 编辑器共享状态 (Shared mutable state for CodeMirror adapter)
// 使用对象属性以便跨模块共享引用

import { Compartment } from "@codemirror/state";

export const ctx = {
  /** @type {import("@codemirror/view").EditorView | null} */
  activeView: null,
  /** 是否正在应用远程操作 (抑制 delta 回声) */
  isRemote: false,
  /** 只读模式 Compartment */
  readOnlyCompartment: new Compartment(),
  /** @type {((json: string) => void) | null} */
  onDeltaCallback: null,
};
