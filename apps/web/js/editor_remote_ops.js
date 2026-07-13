// js/editor_remote_ops.js
// 远程操作处理: applyRemoteContent / applyRemoteOp / applyRemoteOpsBatch / scroll / readonly

import { EditorView } from "@codemirror/view";
import { EditorState, Transaction } from "@codemirror/state";
import { ctx } from "./editor_state.js";

const remoteHistoryAnnotation = Transaction.addToHistory.of(false);

export function getEditorContent() {
  return ctx.activeView ? ctx.activeView.state.doc.toString() : null;
}

export function applyRemoteContent(text) {
  if (!ctx.activeView) return false;
  ctx.isRemote = true;
  try {
    ctx.activeView.dispatch({
      changes: {
        from: 0,
        to: ctx.activeView.state.doc.length,
        insert: String(text ?? ""),
      },
      annotations: remoteHistoryAnnotation,
    });
    return true;
  } catch (e) {
    console.error("applyRemoteContent Error:", e);
    return false;
  } finally {
    ctx.isRemote = false;
  }
}

export function applyRemoteOp(op_json) {
  if (!ctx.activeView) return false;
  ctx.isRemote = true;
  try {
    const op = JSON.parse(op_json);
    if (op.Insert) {
      const pos = op.Insert.pos;
      ensureValidRange(pos, pos);
      ctx.activeView.dispatch({
        changes: { from: pos, insert: op.Insert.content },
        annotations: remoteHistoryAnnotation,
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
        annotations: remoteHistoryAnnotation,
      });
    } else {
      throw new TypeError(`Unsupported remote op: ${JSON.stringify(op)}`);
    }
    return true;
  } catch (e) {
    console.error("applyRemoteOp Error:", e);
    return false;
  } finally {
    ctx.isRemote = false;
  }
}

export function applyRemoteOpsBatch(ops_json) {
  if (!ctx.activeView) return false;
  ctx.isRemote = true;
  try {
    const ops = JSON.parse(ops_json);
    if (!Array.isArray(ops) || ops.length === 0) return true;
    const specs = buildRemoteBatchSpecs(
      ops,
      ctx.activeView.state.doc.length,
      remoteHistoryAnnotation,
    );
    ctx.activeView.dispatch(...specs);
    return true;
  } catch (e) {
    console.error("applyRemoteOpsBatch Error:", e);
    return false;
  } finally {
    ctx.isRemote = false;
  }
}

export function buildRemoteBatchSpecs(ops, initialLength, historyAnnotation) {
  if (!Array.isArray(ops)) {
    throw new TypeError("Remote op batch must be an array");
  }
  if (!Number.isSafeInteger(initialLength) || initialLength < 0) {
    throw new RangeError(`Invalid initial document length ${initialLength}`);
  }
  let virtualLength = initialLength;
  const specs = [];
  for (const op of ops) {
    if (!op || typeof op !== "object" || Array.isArray(op)) {
      throw new TypeError(`Unsupported remote op: ${JSON.stringify(op)}`);
    }
    if (op.Insert && !op.Delete) {
      const pos = op.Insert.pos;
      const content = op.Insert.content;
      if (typeof content !== "string") {
        throw new TypeError("Remote insert content must be a string");
      }
      ensureValidRangeForLength(pos, pos, virtualLength);
      specs.push({
        changes: { from: pos, insert: content },
        annotations: historyAnnotation,
        sequential: true,
      });
      virtualLength += content.length;
    } else if (op.Delete && !op.Insert) {
      const from = op.Delete.pos;
      const len = op.Delete.len;
      if (!Number.isSafeInteger(len) || len < 0) {
        throw new RangeError(`Invalid remote delete length ${len}`);
      }
      const to = from + len;
      ensureValidRangeForLength(from, to, virtualLength);
      specs.push({
        changes: {
          from,
          to,
          insert: "",
        },
        annotations: historyAnnotation,
        sequential: true,
      });
      virtualLength -= to - from;
    } else {
      throw new TypeError(`Unsupported remote op: ${JSON.stringify(op)}`);
    }
  }
  return specs;
}

function ensureValidRange(from, to) {
  const length = ctx.activeView.state.doc.length;
  ensureValidRangeForLength(from, to, length);
}

function ensureValidRangeForLength(from, to, length) {
  if (
    !Number.isSafeInteger(from) ||
    !Number.isSafeInteger(to) ||
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
  if (!ctx.activeView) return false;
  ctx.activeView.dispatch({
    effects: ctx.readOnlyCompartment.reconfigure(
      EditorState.readOnly.of(readOnly),
    ),
  });
  ctx.activeView.contentDOM.setAttribute(
    "contenteditable",
    (!readOnly).toString(),
  );
  return true;
}

export function setReadOnlyForHost(expectedHost, readOnly) {
  if (!expectedHost || ctx.activeHost !== expectedHost) return false;
  return setReadOnly(readOnly);
}
