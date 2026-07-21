const assert = require("node:assert/strict");
const path = require("node:path");
const esbuild = require("esbuild");
const { EditorSelection, EditorState } = require("@codemirror/state");

async function loadModules() {
  const result = await esbuild.build({
    stdin: {
      contents: [
        'export * from "./fenced_ranges.js";',
        'export * from "./render_range_index.js";',
        'export * from "./render_effects.js";',
        'export * from "./math.js";',
        'export * from "./table.js";',
        'export * from "./mermaid.js";',
      ].join("\n"),
      resolveDir: __dirname,
      sourcefile: "render_companion_test_entry.js",
    },
    bundle: true,
    format: "cjs",
    platform: "node",
    external: ["@codemirror/*"],
    write: false,
  });
  const module = { exports: {} };
  new Function("module", "exports", "require", result.outputFiles[0].text)(
    module,
    module.exports,
    require,
  );
  return module.exports;
}

function decorationRecords(state, field) {
  const records = [];
  state.field(field).decorations.between(0, state.doc.length, (from, to, value) => {
    records.push({
      from,
      to,
      key: value.spec.deveRenderKey,
      mode: value.spec.deveRenderMode,
    });
  });
  return records;
}

function nextTurn() {
  return new Promise((resolve) => setImmediate(resolve));
}

(async () => {
  const modules = await loadModules();
  const {
    COMPANION_DEBOUNCE_MS,
    LatestTaskCoordinator,
    MathWidget,
    MAX_COMPANION_MATH_LENGTH,
    MAX_COMPANION_MERMAID_LENGTH,
    MAX_COMPANION_TABLE_CELLS,
    MAX_COMPANION_TABLE_LENGTH,
    MermaidWidget,
    TableWidget,
    buildRenderRangeIndex,
    mathStateField,
    mermaidStateField,
    renderFailureEffect,
    renderRangeIndexField,
    renderThemeEffect,
    scanFencedRanges,
    tableStateField,
  } = modules;

  const fencedFixture = [
    "```mermaid",
    "| fake | table |",
    "| --- | --- |",
    "graph TD",
    "```",
    "~~~js",
    "$$not_math$$",
    "~~~",
    "````text",
    "$still_not_math$",
    "`````",
    "$$a^2$$",
    "| head | value |",
    "| --- | --- |",
    "| one | two |",
  ].join("\n");
  const index = buildRenderRangeIndex(fencedFixture);
  assert.equal(index.ranges("mermaid").length, 1);
  assert.deepEqual(index.ranges("math").map((range) => range.sourceText), ["$$a^2$$"]);
  assert.equal(index.ranges("table").length, 1, "table parser must ignore fenced rows");
  assert.equal(index.ranges("table")[0].data.body.length, 1);

  const unclosed = "~~~js\n$$not_math$$";
  assert.equal(scanFencedRanges(unclosed)[0].closed, false);
  assert.equal(buildRenderRangeIndex(unclosed).ranges("math").length, 0);
  assert.equal(
    buildRenderRangeIndex("$$before\n```text\n$$inside fence\n```").ranges("math").length,
    0,
    "math ranges must not cross into a fenced block",
  );
  assert.equal(
    buildRenderRangeIndex("$$\n| fake | table |\n| --- | --- |\n$$").ranges("table").length,
    0,
    "table ranges must not overlap block math",
  );
  assert.equal(
    scanFencedRanges("prefix ```mermaid\ngraph TD").length,
    0,
    "non-line-start marker must not open a fence",
  );
  assert.equal(
    scanFencedRanges("    ```mermaid\ngraph TD\n    ```").length,
    0,
    "four-space indentation must not open a fence",
  );
  const escapedMathClose = String.raw`$$a \$$ b$$`;
  assert.deepEqual(
    buildRenderRangeIndex(escapedMathClose).ranges("math").map((range) => range.sourceText),
    [escapedMathClose],
    "escaped block delimiters must not close math",
  );
  const codeSpanMathClose = "$$a `$$` b$$";
  assert.deepEqual(
    buildRenderRangeIndex(codeSpanMathClose).ranges("math").map((range) => range.sourceText),
    [codeSpanMathClose],
    "block delimiters inside inline code must not close math",
  );
  const unmatchedCodeMath = "$$a ` b$$";
  assert.deepEqual(
    buildRenderRangeIndex(unmatchedCodeMath).ranges("math").map((range) => range.sourceText),
    [unmatchedCodeMath],
    "an unmatched backtick run must remain ordinary block-math content",
  );
  assert.deepEqual(
    buildRenderRangeIndex("` ordinary $$x$$").ranges("math").map((range) => range.sourceText),
    ["$$x$$"],
    "an unmatched text backtick must not hide later math",
  );
  const escapedRunPrefix = "\\" + "``" + "$hidden$" + "` $$shown$$";
  assert.deepEqual(
    buildRenderRangeIndex(escapedRunPrefix).ranges("math").map((range) => range.sourceText),
    ["$$shown$$"],
    "a suffix run left after an escaped backtick must still protect inline-code math",
  );
  const escapedRunInsideBlock = "$$a " + "\\" + "``" + "$$hidden$$" + "` b$$";
  assert.deepEqual(
    buildRenderRangeIndex(escapedRunInsideBlock).ranges("math").map((range) => range.sourceText),
    [escapedRunInsideBlock],
    "a suffix code-span run must protect block delimiters after an escaped backtick",
  );
  const manyUnmatchedRuns = "$$" + Array.from(
    { length: 128 },
    (_value, i) => `${"`".repeat(i + 1)}x`,
  ).join(" ") + "$$";
  assert.deepEqual(
    buildRenderRangeIndex(manyUnmatchedRuns).ranges("math").map((range) => range.sourceText),
    [manyUnmatchedRuns],
    "many unmatched backtick lengths must preserve a single block range",
  );

  const adjacentMathDoc = "$$a$$$$b$$";
  const adjacentIndex = buildRenderRangeIndex(adjacentMathDoc);
  const adjacentRanges = adjacentIndex.ranges("math");
  assert.equal(adjacentRanges.length, 2);
  const adjacentState = EditorState.create({
    doc: adjacentMathDoc,
    selection: { anchor: adjacentRanges[1].from },
    extensions: [renderRangeIndexField, mathStateField],
  });
  assert.deepEqual(
    decorationRecords(adjacentState, mathStateField)
      .filter((item) => item.mode === "companion")
      .map((item) => item.key),
    [adjacentRanges[1].key],
    "a cursor at an adjacent half-open boundary must own exactly one companion",
  );

  const mathFrom = fencedFixture.indexOf("$$a^2$$");
  const tableFrom = fencedFixture.indexOf("| head | value |");
  let state = EditorState.create({
    doc: fencedFixture,
    selection: { anchor: mathFrom + 3 },
    extensions: [
      EditorState.allowMultipleSelections.of(true),
      renderRangeIndexField,
      mathStateField,
      tableStateField,
      mermaidStateField,
    ],
  });
  const cachedIndex = state.field(renderRangeIndexField);
  assert.equal(
    decorationRecords(state, mathStateField).filter((item) => item.mode === "companion").length,
    1,
  );

  state = state.update({ selection: { anchor: mathFrom + 2, head: mathFrom + 5 } }).state;
  assert.strictEqual(state.field(renderRangeIndexField), cachedIndex);
  assert.equal(decorationRecords(state, mathStateField).some((item) => item.mode === "companion"), false);

  const mathRange = state.field(renderRangeIndexField).ranges("math")[0];
  const tableRange = state.field(renderRangeIndexField).ranges("table")[0];
  assert.equal(
    new MathWidget(mathRange, "replace").eq(new MathWidget({ ...mathRange, key: "shifted" }, "replace")),
    false,
    "widget identity must include the source range key",
  );
  assert.equal(
    new TableWidget(tableRange, "replace").eq(new TableWidget({ ...tableRange, key: "shifted" }, "replace")),
    false,
    "table widgets must not retain stale click ranges after positional edits",
  );
  state = state.update({
    selection: EditorSelection.create([
      EditorSelection.cursor(tableFrom + 2),
      EditorSelection.cursor(mathFrom + 2),
    ], 0),
  }).state;
  assert.strictEqual(state.field(renderRangeIndexField), cachedIndex);
  assert.equal(
    decorationRecords(state, tableStateField).filter((item) => item.mode === "companion").length,
    1,
    "only the collapsed main selection produces a companion",
  );
  assert.equal(
    decorationRecords(state, mathStateField).some((item) => item.key === mathRange.key),
    false,
    "secondary selections still reveal source",
  );

  state = state.update({ selection: { anchor: fencedFixture.length } }).state;
  const mermaidRange = state.field(renderRangeIndexField).ranges("mermaid")[0];
  assert.equal(
    new MermaidWidget(mermaidRange, "replace", "day").eq(
      new MermaidWidget({ ...mermaidRange, key: "shifted" }, "replace", "day"),
    ),
    false,
    "Mermaid widgets must not retain stale asynchronous range ownership",
  );

  for (const [field, range] of [[mathStateField, mathRange], [tableStateField, tableRange]]) {
    state = state.update({ effects: renderFailureEffect.of({
      kind: range.kind,
      from: range.from,
      to: range.to,
      sourceText: range.sourceText,
    }) }).state;
    assert.equal(
      decorationRecords(state, field).some((item) => item.key === range.key),
      false,
      `${range.kind} replace failure must restore source`,
    );
    state = state.update({ effects: renderThemeEffect.of("day") }).state;
    assert.equal(
      decorationRecords(state, field).some((item) => item.key === range.key),
      true,
      `${range.kind} theme refresh must retry a failed renderer`,
    );
  }

  assert.equal(
    decorationRecords(state, mermaidStateField).some((item) => item.key === mermaidRange.key),
    true,
  );
  const beforeFailureDoc = state.doc;
  state = state.update({ effects: renderFailureEffect.of({
    kind: mermaidRange.kind,
    from: mermaidRange.from,
    to: mermaidRange.to,
    sourceText: mermaidRange.sourceText,
  }) }).state;
  assert.strictEqual(state.doc, beforeFailureDoc);
  assert.equal(
    decorationRecords(state, mermaidStateField).some((item) => item.key === mermaidRange.key),
    false,
    "replace renderer failure must restore source",
  );

  state = state.update({ selection: { anchor: mermaidRange.from + 2 } }).state;
  assert.equal(
    decorationRecords(state, mermaidStateField).some((item) => (
      item.key === mermaidRange.key && item.mode === "companion"
    )),
    true,
    "an active companion owns its own compact failure state",
  );
  state = state.update({ selection: { anchor: fencedFixture.length } }).state;

  const beforeThemeIndex = state.field(renderRangeIndexField);
  state = state.update({ effects: renderThemeEffect.of("night") }).state;
  assert.strictEqual(state.field(renderRangeIndexField), beforeThemeIndex);
  assert.equal(state.doc.toString(), fencedFixture);
  assert.equal(
    decorationRecords(state, mermaidStateField).some((item) => item.key === mermaidRange.key),
    true,
    "theme refresh clears stale renderer failure",
  );

  assert.equal(COMPANION_DEBOUNCE_MS, 200);
  assert.equal(MAX_COMPANION_MATH_LENGTH, 8192);
  assert.equal(MAX_COMPANION_MERMAID_LENGTH, 32768);
  assert.equal(MAX_COMPANION_TABLE_LENGTH, 131072);
  assert.equal(MAX_COMPANION_TABLE_CELLS, 2000);
  assert.ok(tableRange.data.header.length > 0);

  const coordinator = new LatestTaskCoordinator();
  const calls = [];
  let releaseFirst;
  const firstDone = new Promise((resolve) => { releaseFirst = resolve; });
  coordinator.enqueue(async () => {
    calls.push("first");
    await firstDone;
  });
  coordinator.enqueue(async () => calls.push("superseded"));
  coordinator.enqueue(async () => calls.push("latest"));
  releaseFirst();
  await nextTurn();
  await nextTurn();
  assert.deepEqual(calls, ["first", "latest"]);

  console.log("render-companion-range-index-and-state: ok");
})().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
