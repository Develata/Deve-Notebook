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
  if (!ctx.activeView) return false;
  ctx.isRemote = true;
  try {
    const ops = JSON.parse(ops_json);
    if (!Array.isArray(ops) || ops.length === 0) return true;
    for (const op of ops) {
      if (op.Insert) {
        const pos = op.Insert.pos;
        ensureValidRange(pos, pos);
        ctx.activeView.dispatch({
          changes: { from: pos, insert: op.Insert.content },
        });
      } else if (op.Delete) {
        const from = op.Delete.pos;
        const to = op.Delete.pos + op.Delete.len;
        ensureValidRange(from, to);
        ctx.activeView.dispatch({
          changes: {
            from,
            to,
            insert: "",
          },
        });
      }
    }
    return true;
  } catch (e) {
    console.error("applyRemoteOpsBatch Error:", e);
    return false;
  } finally {
    ctx.isRemote = false;
  }
}

function ensureValidRange(from, to) {
  const length = ctx.activeView.state.doc.length;
  if (
    !Number.isInteger(from) ||
    !Number.isInteger(to) ||
    from < 0 ||
    to < from ||
    to > length
  ) {
    throw new RangeError(`Invalid remote op range ${from}..${to} for ${length}`);
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
