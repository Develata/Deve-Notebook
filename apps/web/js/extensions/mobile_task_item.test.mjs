import assert from "node:assert/strict";
import test from "node:test";

import { EditorState } from "@codemirror/state";
import {
  insertNewlineContinueMarkup,
  markdown,
  markdownLanguage,
} from "@codemirror/lang-markdown";
import { GFM } from "@lezer/markdown";
import {
  continueToolbarTaskItem,
  insertToolbarTaskItem,
  mobileTaskItemExtension,
  toolbarTaskMarkerField,
} from "./mobile_task_item.js";

function editorHarness(doc = "") {
  let state = EditorState.create({
    doc,
    extensions: [
      markdown({ base: markdownLanguage, extensions: [GFM] }),
      mobileTaskItemExtension,
    ],
  });
  return {
    get state() {
      return state;
    },
    dispatch(spec) {
      state = spec?.state instanceof EditorState ? spec.state : state.update(spec).state;
    },
    focus() {},
  };
}

test("toolbar task Enter continues once, then standard empty-list exit remains", () => {
  const view = editorHarness();

  assert.equal(insertToolbarTaskItem(view), true);
  assert.equal(view.state.doc.toString(), "- [ ] ");
  assert.equal(continueToolbarTaskItem(view), true);
  assert.equal(view.state.doc.toString(), "- [ ] \n- [ ] ");
  assert.equal(view.state.field(toolbarTaskMarkerField), null);

  const standardView = editorHarness(view.state.doc.toString());
  standardView.dispatch({ selection: { anchor: standardView.state.doc.length } });
  assert.equal(insertNewlineContinueMarkup(standardView), true);

  assert.equal(continueToolbarTaskItem(view), false);
  assert.equal(insertNewlineContinueMarkup(view), true);
  assert.equal(view.state.doc.toString(), standardView.state.doc.toString());
  assert.equal(view.state.doc.toString().includes("\u200b"), false);
});

test("typing retires the one-shot toolbar task marker", () => {
  const view = editorHarness();
  insertToolbarTaskItem(view);
  const cursor = view.state.selection.main.head;
  view.dispatch({ changes: { from: cursor, insert: "done" } });

  assert.equal(view.state.field(toolbarTaskMarkerField), null);
  assert.equal(continueToolbarTaskItem(view), false);
});

test("unrelated changes before the task map the one-shot marker", () => {
  const view = editorHarness("intro\n");
  view.dispatch({ selection: { anchor: view.state.doc.length } });
  insertToolbarTaskItem(view);
  const oldMarker = view.state.field(toolbarTaskMarkerField);

  view.dispatch({ changes: { from: 0, insert: "# " } });

  assert.equal(view.state.field(toolbarTaskMarkerField), oldMarker + 2);
  assert.equal(continueToolbarTaskItem(view), true);
  assert.equal(view.state.doc.toString(), "# intro\n- [ ] \n- [ ] ");
});

test("rewriting the task line retires the mapped marker", () => {
  const view = editorHarness();
  insertToolbarTaskItem(view);

  view.dispatch({ changes: { from: 2, to: 5, insert: "[x]" } });

  assert.equal(view.state.field(toolbarTaskMarkerField), null);
  assert.equal(continueToolbarTaskItem(view), false);
});
