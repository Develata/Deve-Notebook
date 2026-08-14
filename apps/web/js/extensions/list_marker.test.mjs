import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import { EditorState } from "@codemirror/state";
import { syntaxTree } from "@codemirror/language";
import { markdown, markdownLanguage } from "@codemirror/lang-markdown";
import { GFM } from "@lezer/markdown";
import { listMarkProjectionKind } from "./list_marker.js";

function listMarkKinds(doc) {
  const state = EditorState.create({
    doc,
    extensions: [markdown({ base: markdownLanguage, extensions: [GFM] })],
  });
  const kinds = [];
  syntaxTree(state).iterate({
    enter(node) {
      if (node.name === "ListMark") {
        kinds.push(listMarkProjectionKind(node.node, state.doc.sliceString(node.from, node.to)));
      }
    },
  });
  return kinds;
}

test("task checkbox is the only visual marker for a task list item", () => {
  assert.deepEqual(listMarkKinds("- [ ] task\n- [x] done"), ["task", "task"]);
});

test("ordinary unordered and ordered list markers retain their projections", () => {
  assert.deepEqual(listMarkKinds("- plain"), ["unordered"]);
  assert.deepEqual(listMarkKinds("3. ordered"), ["ordered"]);
});

test("inactive task list mark is hidden instead of drawing a second marker", async () => {
  const source = await readFile(new URL("./list_marker.js", import.meta.url), "utf8");
  const taskBranch = source.slice(source.indexOf('projectionKind === "task"'));

  assert.match(taskBranch, /Decoration\.replace\(\{\}\)\.range\(node\.from, node\.to\)/);
});
