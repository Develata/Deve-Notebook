const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");

const sourcePath = path.join(__dirname, "block_styling.js");
const source = fs.readFileSync(sourcePath, "utf8");
const purePrefix = source.split("/**")[0].replace(/^import .*$/gm, "");
const context = {
  findMathRanges: (doc) => {
    const blockStart = doc.indexOf("$$");
    if (blockStart !== -1) {
      const blockEnd = doc.indexOf("$$", blockStart + 2);
      if (blockEnd !== -1) return [{ from: blockStart, to: blockEnd + 2 }];
    }
    const inlineStart = doc.indexOf("$");
    if (inlineStart === -1) return [];
    const inlineEnd = doc.indexOf("$", inlineStart + 1);
    return inlineEnd === -1 ? [] : [{ from: inlineStart, to: inlineEnd + 1 }];
  },
  findFrontmatterRange: (doc) => {
    if (!doc.startsWith("---\n")) return null;
    const end = doc.indexOf("\n---", 4);
    return end === -1 ? null : { from: 0, to: end + 4 };
  },
};
const hybridSourcePath = path.join(__dirname, "hybrid.js");
const hybridSource = fs.readFileSync(hybridSourcePath, "utf8");
const typographyCssPath = path.join(__dirname, "../../style/_typography.css");
const baseCssPath = path.join(__dirname, "../../style/_base.css");
const typographyCss = fs.readFileSync(typographyCssPath, "utf8");
const baseCss = fs.readFileSync(baseCssPath, "utf8");

vm.runInNewContext(
  `${purePrefix}
globalThis.atxHeadingLevel = atxHeadingLevel;
globalThis.headingLineClass = headingLineClass;
globalThis.protectedHeadingRanges = protectedHeadingRanges;
globalThis.textMayContainProtectedDelimiter = textMayContainProtectedDelimiter;
globalThis.mapProtectedRanges = mapProtectedRanges;
globalThis.frontmatterCandidateBoundary = frontmatterCandidateBoundary;
globalThis.headingLineClassForLine = headingLineClassForLine;`,
  context,
  { filename: sourcePath }
);

const {
  atxHeadingLevel,
  headingLineClass,
  protectedHeadingRanges,
  textMayContainProtectedDelimiter,
  mapProtectedRanges,
  frontmatterCandidateBoundary,
  headingLineClassForLine,
} = context;
assert.equal(typeof atxHeadingLevel, "function");
assert.equal(typeof headingLineClass, "function");
assert.equal(typeof protectedHeadingRanges, "function");
assert.equal(typeof textMayContainProtectedDelimiter, "function");
assert.equal(typeof mapProtectedRanges, "function");
assert.equal(typeof frontmatterCandidateBoundary, "function");
assert.equal(typeof headingLineClassForLine, "function");

function fakeDoc(text) {
  const parts = text.split("\n");
  let pos = 0;
  const lines = parts.map((line) => {
    const from = pos;
    const to = from + line.length;
    pos = to + 1;
    return { from, to, text: line };
  });
  return {
    length: text.length,
    lines: lines.length,
    line: (lineNo) => lines[lineNo - 1],
    sliceString: (from, to) => text.slice(from, to),
  };
}

const cases = [
  ["#", false, 1],
  ["# s", false, 1],
  ["# s", true, 1],
  ["# h1", false, 1],
  ["#\th1", false, 1],
  ["# 申话", false, 1],
  ["## h2", false, 2],
  ["## s", true, 2],
  ["### h3", false, 3],
  ["### s", true, 3],
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

assert.equal(
  headingLineClassForLine({ from: 0, text: "# s" }, false, []),
  "cm-h1 cm-heading-line cm-heading-line-1"
);
assert.equal(
  headingLineClassForLine({ from: 3, text: "  ## s" }, false, [{ from: 0, to: 12 }]),
  null,
  "heading opener inside protected math/frontmatter range must not be styled"
);
assert.equal(
  headingLineClassForLine({ from: 12, text: "# s" }, false, [{ from: 0, to: 12 }]),
  "cm-h1 cm-heading-line cm-heading-line-1",
  "range end is exclusive so following headings remain styled"
);
assert.equal(
  JSON.stringify(protectedHeadingRanges("$$\n# not heading\n$$")),
  JSON.stringify([{ from: 0, to: 19 }])
);
assert.equal(
  JSON.stringify(protectedHeadingRanges("$# not heading$")),
  JSON.stringify([{ from: 0, to: 15 }])
);
assert.equal(
  JSON.stringify(protectedHeadingRanges("---\n# comment\n---\n# heading")),
  JSON.stringify([{ from: 0, to: 17 }])
);
assert.equal(textMayContainProtectedDelimiter("plain edit"), false);
assert.equal(textMayContainProtectedDelimiter("$"), true);
assert.equal(textMayContainProtectedDelimiter("$$"), true);
assert.equal(textMayContainProtectedDelimiter("`"), true);
assert.equal(textMayContainProtectedDelimiter("\\"), true);
assert.equal(textMayContainProtectedDelimiter("---"), true);
assert.equal(textMayContainProtectedDelimiter("\n"), true);
assert.equal(
  JSON.stringify(
    mapProtectedRanges([{ from: 10, to: 20 }], {
      mapPos: (pos) => (pos >= 10 ? pos + 3 : pos),
    })
  ),
  JSON.stringify([{ from: 13, to: 23 }])
);
assert.equal(frontmatterCandidateBoundary(fakeDoc("no frontmatter\n# heading")), 0);
assert.equal(frontmatterCandidateBoundary(fakeDoc("---\nplain\n---\n# heading")), 9);
assert.equal(
  frontmatterCandidateBoundary(fakeDoc("---\nplain: value\n---\n# heading")),
  20
);

assert.doesNotMatch(
  hybridSource,
  /cm-heading-line/,
  "hybrid plugin must not duplicate the block styling heading line projection"
);

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
assert.match(
  headingLineRule,
  /padding-top:\s*0;/,
  "heading line must not grow or shrink from source-mode padding"
);
assert.match(
  headingLineRule,
  /padding-bottom:\s*0;/,
  "heading line must not grow or shrink from source-mode padding"
);

const activeHeadingLineRule = cssRule(
  typographyCss,
  ".cm-content .cm-line.cm-heading-line.cm-activeLine"
);
assert.match(
  activeHeadingLineRule,
  /font-size:\s*var\(--deve-heading-font-size\);/,
  "active source heading line must keep heading font-size"
);
assert.match(
  activeHeadingLineRule,
  /line-height:\s*var\(--deve-heading-inline-line-height\);/,
  "active source heading line must keep heading line-height"
);
assert.match(
  activeHeadingLineRule,
  /min-height:\s*var\(--deve-heading-line-box\);/,
  "active source heading line must keep heading line box"
);

for (const selector of [
  ".cm-content .cm-line.cm-heading-line span",
  ".cm-content .cm-line.cm-heading-line span span",
  ".cm-content .cm-line.cm-heading-line > span",
  ".cm-content .cm-line.cm-heading-line .cm-heading-mark",
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
