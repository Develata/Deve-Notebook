import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import { EditorState } from "@codemirror/state";
import { syntaxTree } from "@codemirror/language";
import { markdown, markdownLanguage } from "@codemirror/lang-markdown";
import { GFM } from "@lezer/markdown";
import { inlineProjectionClassForNodeName } from "./hybrid.js";

test("strong and emphasis syntax use explicit stable projection classes", () => {
  const state = EditorState.create({
    doc: "normal **123** and _italic_",
    extensions: [markdown({ base: markdownLanguage, extensions: [GFM] })],
  });
  const projections = [];
  syntaxTree(state).iterate({
    enter(node) {
      const projectionClass = inlineProjectionClassForNodeName(node.name);
      if (projectionClass) projections.push([node.name, projectionClass]);
    },
  });

  assert.deepEqual(projections, [
    ["StrongEmphasis", "cm-strong"],
    ["Emphasis", "cm-em"],
  ]);
});

test("strong projection keeps an OEM-font-visible weight fallback", async () => {
  const css = await readFile(new URL("../../style/_typography.css", import.meta.url), "utf8");
  const rule = css.match(/\.cm-strong\s*\{(?<body>[^}]*)\}/s)?.groups?.body ?? "";

  assert.match(rule, /font-weight:\s*(?:800|900)/);
  assert.match(rule, /-webkit-text-stroke:/);
});

test("Markdown content font stack prefers Times New Roman and FangSong with serif fallback", async () => {
  const css = await readFile(new URL("../../style/_base.css", import.meta.url), "utf8");
  const rule = css.match(/\.cm-content\s*\{(?<body>[^}]*)\}/s)?.groups?.body ?? "";
  const orderedFonts = [
    '"Times New Roman"',
    '"FangSong_GB2312"',
    "FangSong",
    '"仿宋"',
    '"Noto Serif CJK SC"',
    '"Source Han Serif SC"',
    "serif",
  ];
  let previous = -1;
  for (const font of orderedFonts) {
    const current = rule.indexOf(font);
    assert.ok(current > previous, `${font} must appear in the expected fallback order`);
    previous = current;
  }
});

test("clickable editor projections opt out of Work Edit drawer swipe admission", async () => {
  const [hybrid, image, math, mermaid, table] = await Promise.all([
    readFile(new URL("./hybrid.js", import.meta.url), "utf8"),
    readFile(new URL("./image.js", import.meta.url), "utf8"),
    readFile(new URL("./math.js", import.meta.url), "utf8"),
    readFile(new URL("./mermaid.js", import.meta.url), "utf8"),
    readFile(new URL("./table.js", import.meta.url), "utf8"),
  ]);

  assert.match(hybrid, /attributes:\s*\{\s*"data-no-edge-swipe":\s*"true"\s*\}/);
  assert.match(image, /container\.dataset\.noEdgeSwipe\s*=\s*"true"/);
  for (const source of [math, mermaid, table]) {
    assert.match(source, /this\.mode === "replace"[^\n]*dataset\.noEdgeSwipe\s*=\s*"true"/);
  }
});
