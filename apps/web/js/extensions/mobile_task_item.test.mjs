import assert from "node:assert/strict";
import test from "node:test";

import { EditorSelection, EditorState } from "@codemirror/state";
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

function editorHarnessWithExtensions(doc = "", extraExtensions = []) {
  let state = EditorState.create({
    doc,
    extensions: [
      markdown({ base: markdownLanguage, extensions: [GFM] }),
      mobileTaskItemExtension,
      ...extraExtensions,
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

function editorHarness(doc = "") {
  return editorHarnessWithExtensions(doc);
}

test("toolbar task Enter continues once, then the generated empty item exits immediately", () => {
  const view = editorHarness();

  assert.equal(insertToolbarTaskItem(view), true);
  assert.equal(view.state.doc.toString(), "- [ ] ");
  assert.equal(continueToolbarTaskItem(view), true);
  assert.equal(view.state.doc.toString(), "- [ ] \n- [ ] ");
  assert.deepEqual(view.state.field(toolbarTaskMarkerField), {
    pos: view.state.doc.length,
    phase: "exit-ready",
  });
  assert.equal(continueToolbarTaskItem(view), true);
  assert.equal(view.state.doc.toString(), "- [ ] \n");
  assert.equal(view.state.field(toolbarTaskMarkerField), null);
  assert.equal(view.state.doc.toString().includes("\u200b"), false);
});

test("ordinary keyboard-created empty task keeps CodeMirror default Enter behavior", () => {
  const view = editorHarness("- [ ] task\n- [ ] ");
  view.dispatch({ selection: { anchor: view.state.doc.length } });

  assert.equal(continueToolbarTaskItem(view), false);
  assert.equal(insertNewlineContinueMarkup(view), true);
  assert.equal(view.state.doc.toString(), "- [ ] task\n\n- [ ] ");
});

test("typing retires the intent-local toolbar task marker", () => {
  const view = editorHarness();
  insertToolbarTaskItem(view);
  const cursor = view.state.selection.main.head;
  view.dispatch({ changes: { from: cursor, insert: "done" } });

  assert.equal(view.state.field(toolbarTaskMarkerField), null);
  assert.equal(continueToolbarTaskItem(view), false);
});

test("unrelated changes before the task map the two-phase marker", () => {
  const view = editorHarness("intro\n");
  view.dispatch({ selection: { anchor: view.state.doc.length } });
  insertToolbarTaskItem(view);
  const oldMarker = view.state.field(toolbarTaskMarkerField);

  view.dispatch({ changes: { from: 0, insert: "# " } });

  assert.deepEqual(view.state.field(toolbarTaskMarkerField), {
    ...oldMarker,
    pos: oldMarker.pos + 2,
  });
  assert.equal(continueToolbarTaskItem(view), true);
  assert.equal(view.state.doc.toString(), "# intro\n- [ ] \n- [ ] ");
});

test("unrelated changes map the exit-ready item without broadening Enter semantics", () => {
  const view = editorHarness("intro\n");
  view.dispatch({ selection: { anchor: view.state.doc.length } });
  insertToolbarTaskItem(view);
  continueToolbarTaskItem(view);
  const oldMarker = view.state.field(toolbarTaskMarkerField);

  view.dispatch({ changes: { from: 0, insert: "# " } });

  assert.deepEqual(view.state.field(toolbarTaskMarkerField), {
    ...oldMarker,
    pos: oldMarker.pos + 2,
  });
  assert.equal(continueToolbarTaskItem(view), true);
  assert.equal(view.state.doc.toString(), "# intro\n- [ ] \n");
});

test("filling the generated item retires exit-ready behavior", () => {
  const view = editorHarness();
  insertToolbarTaskItem(view);
  continueToolbarTaskItem(view);
  const cursor = view.state.selection.main.head;

  view.dispatch({ changes: { from: cursor, insert: "next" } });

  assert.equal(view.state.field(toolbarTaskMarkerField), null);
  assert.equal(continueToolbarTaskItem(view), false);
  assert.equal(view.state.doc.toString(), "- [ ] \n- [ ] next");
});

test("selection leaving either phase retires toolbar-specific Enter behavior", () => {
  for (const phase of ["continue-ready", "exit-ready"]) {
    const view = editorHarness();
    insertToolbarTaskItem(view);
    if (phase === "exit-ready") continueToolbarTaskItem(view);
    const emptyTaskEnd = view.state.selection.main.head;

    view.dispatch({ selection: { anchor: 0 } });
    assert.equal(view.state.field(toolbarTaskMarkerField), null);
    view.dispatch({ selection: { anchor: emptyTaskEnd } });
    assert.equal(continueToolbarTaskItem(view), false);
  }
});

test("transaction filters map a newly installed marker into the final document", () => {
  let prefixNextTransaction = true;
  const prefixingFilter = EditorState.transactionFilter.of((transaction) => {
    if (!prefixNextTransaction) return transaction;
    prefixNextTransaction = false;
    return [
      transaction,
      { changes: { from: 0, insert: "\n" }, sequential: true },
    ];
  });
  const view = editorHarnessWithExtensions("", [prefixingFilter]);

  assert.equal(insertToolbarTaskItem(view), true);
  assert.equal(view.state.doc.toString(), "\n- [ ] ");
  assert.deepEqual(view.state.field(toolbarTaskMarkerField), {
    pos: view.state.doc.length,
    phase: "continue-ready",
  });
  assert.equal(continueToolbarTaskItem(view), true);
  assert.equal(view.state.doc.toString(), "\n- [ ] \n- [ ] ");
});

test("multiple selections retire toolbar-specific Enter behavior without collapsing ranges", () => {
  const view = editorHarnessWithExtensions("prefix\n", [
    EditorState.allowMultipleSelections.of(true),
  ]);
  view.dispatch({ selection: { anchor: view.state.doc.length } });
  insertToolbarTaskItem(view);
  view.dispatch({
    selection: EditorSelection.create([
      EditorSelection.cursor(0),
      EditorSelection.cursor(view.state.doc.length),
    ], 1),
  });
  const rangesBefore = view.state.selection.ranges.map(({ anchor, head }) => ({ anchor, head }));

  assert.equal(view.state.field(toolbarTaskMarkerField), null);
  assert.equal(continueToolbarTaskItem(view), false);
  assert.deepEqual(
    view.state.selection.ranges.map(({ anchor, head }) => ({ anchor, head })),
    rangesBefore,
  );
});

test("task intent rejects a nonempty selection without replacing content", () => {
  const view = editorHarness("abc");
  view.dispatch({ selection: { anchor: 0, head: 3 } });
  const selectionBefore = view.state.selection;

  assert.equal(insertToolbarTaskItem(view), false);
  assert.equal(view.state.doc.toString(), "abc");
  assert.deepEqual(view.state.selection, selectionBefore);
  assert.equal(view.state.field(toolbarTaskMarkerField), null);
});

test("task intent rejects preexisting multiple selections without collapsing them", () => {
  const view = editorHarnessWithExtensions("abc\n", [
    EditorState.allowMultipleSelections.of(true),
  ]);
  view.dispatch({
    selection: EditorSelection.create([
      EditorSelection.cursor(0),
      EditorSelection.cursor(view.state.doc.length),
    ], 1),
  });
  const rangesBefore = view.state.selection.ranges.map(({ anchor, head }) => ({ anchor, head }));

  assert.equal(insertToolbarTaskItem(view), false);
  assert.equal(view.state.doc.toString(), "abc\n");
  assert.deepEqual(
    view.state.selection.ranges.map(({ anchor, head }) => ({ anchor, head })),
    rangesBefore,
  );
  assert.equal(view.state.field(toolbarTaskMarkerField), null);
});

test("rewriting the task line retires the mapped marker", () => {
  const view = editorHarness();
  insertToolbarTaskItem(view);

  view.dispatch({ changes: { from: 2, to: 5, insert: "[x]" } });

  assert.equal(view.state.field(toolbarTaskMarkerField), null);
  assert.equal(continueToolbarTaskItem(view), false);
});
