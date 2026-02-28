// js/editor_remote_ops.js
// 远程操作处理: applyRemoteContent / applyRemoteOp / applyRemoteOpsBatch / scroll / readonly

import { EditorView } from "@codemirror/view";
import { EditorState } from "@codemirror/state";
import { ctx } from "./editor_state.js";

export function getEditorContent() {
  return ctx.activeView ? ctx.activeView.state.doc.toString() : "";
}

export function applyRemoteContent(text) {
  if (ctx.activeView) {
    ctx.isRemote = true;
    try {
      ctx.activeView.dispatch({
        changes: {
          from: 0,
          to: ctx.activeView.state.doc.length,
          insert: text,
        },
      });
    } catch (e) {
      console.error("applyRemoteContent Error:", e);
    } finally {
      ctx.isRemote = false;
    }
  }
}

export function applyRemoteOp(op_json) {
  if (ctx.activeView) {
    ctx.isRemote = true;
    try {
      const op = JSON.parse(op_json);
      if (op.Insert) {
        ctx.activeView.dispatch({
          changes: { from: op.Insert.pos, insert: op.Insert.content },
        });
      } else if (op.Delete) {
        ctx.activeView.dispatch({
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
      ctx.isRemote = false;
    }
  }
}

export function applyRemoteOpsBatch(ops_json) {
  if (ctx.activeView) {
    ctx.isRemote = true;
    try {
      const ops = JSON.parse(ops_json);
      if (!Array.isArray(ops) || ops.length === 0) return;

      // Collect all changes and apply in a single dispatch to avoid O(N²)
      const changes = [];
      for (const op of ops) {
        if (op.Insert) {
          changes.push({ from: op.Insert.pos, insert: op.Insert.content });
        } else if (op.Delete) {
          changes.push({
            from: op.Delete.pos,
            to: op.Delete.pos + op.Delete.len,
            insert: "",
          });
        }
      }
      if (changes.length > 0) {
        ctx.activeView.dispatch({ changes });
      }
    } catch (e) {
      console.error("applyRemoteOpsBatch Error:", e);
    } finally {
      ctx.isRemote = false;
    }
  }
}

export function scrollGlobal(lineNumber) {
  if (!ctx.activeView || !ctx.activeView.state) return;

  const doc = ctx.activeView.state.doc;
  const lines = doc.lines;
  if (lineNumber < 1) lineNumber = 1;
  if (lineNumber > lines) lineNumber = lines;

  const line = doc.line(lineNumber);

  ctx.activeView.dispatch({
    effects: [
      EditorView.scrollIntoView(line.from, { y: "start", yMargin: 20 }),
    ],
    selection: { anchor: line.from },
  });
  ctx.activeView.focus();
}

export function setReadOnly(readOnly) {
  if (ctx.activeView) {
    ctx.activeView.dispatch({
      effects: ctx.readOnlyCompartment.reconfigure(
        EditorState.readOnly.of(readOnly),
      ),
    });
    ctx.activeView.contentDOM.setAttribute(
      "contenteditable",
      (!readOnly).toString(),
    );
  }
}
