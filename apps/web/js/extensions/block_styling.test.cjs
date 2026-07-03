const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");

const sourcePath = path.join(__dirname, "block_styling.js");
const source = fs.readFileSync(sourcePath, "utf8");
const purePrefix = source.split("/**")[0].replace(/^import .*$/gm, "");
const context = {};
const hybridSourcePath = path.join(__dirname, "hybrid.js");
const hybridSource = fs.readFileSync(hybridSourcePath, "utf8");
const hybridPurePrefix = hybridSource.split("/**")[0].replace(/^import .*$/gm, "");
const hybridContext = {};
const typographyCssPath = path.join(__dirname, "../../style/_typography.css");
const baseCssPath = path.join(__dirname, "../../style/_base.css");
const typographyCss = fs.readFileSync(typographyCssPath, "utf8");
const baseCss = fs.readFileSync(baseCssPath, "utf8");

vm.runInNewContext(
  `${purePrefix}\nglobalThis.atxHeadingLevel = atxHeadingLevel; globalThis.headingLineClass = headingLineClass;`,
  context,
  { filename: sourcePath }
);

const { atxHeadingLevel, headingLineClass } = context;
assert.equal(typeof atxHeadingLevel, "function");
assert.equal(typeof headingLineClass, "function");

vm.runInNewContext(
  `${hybridPurePrefix}\nglobalThis.atxHeadingLevelFromLine = atxHeadingLevelFromLine;`,
  hybridContext,
  { filename: hybridSourcePath }
);

const { atxHeadingLevelFromLine } = hybridContext;
assert.equal(typeof atxHeadingLevelFromLine, "function");

const cases = [
  ["#", false, 1],
  ["# s", false, 1],
  ["# h1", false, 1],
  ["#\th1", false, 1],
  ["## h2", false, 2],
  ["### h3", false, 3],
  ["###### h6", false, 6],
  ["####### too many", false, null],
  ["    # code", false, null],
  ["#ascii", true, null],
  ["#申话", false, null],
  ["#申话", true, 1],
  ["##标题", true, 2],
];

for (const [line, isActiveLine, expected] of cases) {
  assert.equal(
    atxHeadingLevel(line, isActiveLine),
    expected,
    `${JSON.stringify(line)} active=${isActiveLine}`
  );
}

for (const level of [1, 2, 3]) {
  assert.equal(
    headingLineClass(level),
    `cm-h${level} cm-heading-line cm-heading-line-${level}`,
    `h${level} must carry the shared heading line projection class`
  );
}

for (const level of [4, 5, 6]) {
  assert.equal(
    headingLineClass(level),
    `cm-h${level}`,
    `h${level} keeps the legacy class until line-level CSS variables exist`
  );
}

const hybridHeadingCases = [
  ["#", "1"],
  ["# s", "1"],
  ["# 申话", "1"],
  ["#\ts", "1"],
  ["## s", "2"],
  ["### s", "3"],
  ["#### s", null],
  ["#ascii", null],
  ["#申话", null],
  ["    # code", null],
];

for (const [line, expected] of hybridHeadingCases) {
  assert.equal(
    atxHeadingLevelFromLine(line),
    expected,
    `hybrid ${JSON.stringify(line)}`
  );
}

function cssRule(css, selector) {
  const start = css.indexOf(`${selector} {`);
  assert.notEqual(start, -1, `${selector} rule missing`);
  const end = css.indexOf("}", start);
  assert.notEqual(end, -1, `${selector} rule closing brace missing`);
  return css.slice(start, end);
}

function cssNumber(rule, property) {
  const match = rule.match(new RegExp(`${property}:\\s*([0-9.]+)(?:[a-z%]+)?\\s*;`));
  assert.ok(match, `${property} missing in ${rule}`);
  return Number(match[1]);
}

const plainContentRule = cssRule(baseCss, ".cm-content");
const plainLineHeight = cssNumber(plainContentRule, "line-height");

for (const selector of [
  ".cm-line.cm-h1",
  ".cm-line.cm-h2",
  ".cm-line.cm-h3",
  ".cm-line.cm-h4",
  ".cm-line.cm-h5",
  ".cm-line.cm-h6",
]) {
  const rule = cssRule(typographyCss, selector);
  const headingFontSize = cssNumber(rule, "font-size");
  const headingLineHeight = cssNumber(rule, "line-height");
  const headingMinHeight = cssNumber(rule, "min-height");

  assert.ok(
    headingFontSize * headingLineHeight > plainLineHeight,
    `${selector} line-height must stay taller than plain text`
  );
  assert.ok(
    headingFontSize * headingMinHeight > plainLineHeight,
    `${selector} min-height must stay taller than plain text`
  );
}

const headingLineRule = cssRule(typographyCss, ".cm-content .cm-line.cm-heading-line");
assert.match(
  headingLineRule,
  /font-size:\s*var\(--deve-heading-font-size\);/,
  "heading line must bind font-size to the line-level projection variable"
);
assert.match(
  headingLineRule,
  /line-height:\s*var\(--deve-heading-inline-line-height\);/,
  "heading line must bind line-height to the line-level projection variable"
);
assert.match(
  headingLineRule,
  /min-height:\s*var\(--deve-heading-line-box\);/,
  "heading line must keep a stable line box"
);

for (const selector of [
  ".cm-content .cm-line.cm-heading-line span",
  ".cm-content .cm-line.cm-heading-line span span",
]) {
  const rule = cssRule(typographyCss, selector);
  assert.match(
    rule,
    /font-size:\s*inherit;/,
    `${selector} must inherit heading font-size`
  );
  assert.match(
    rule,
    /line-height:\s*inherit;/,
    `${selector} must inherit heading line-height`
  );
}

console.log("block-styling-heading-rules-and-css: ok");
