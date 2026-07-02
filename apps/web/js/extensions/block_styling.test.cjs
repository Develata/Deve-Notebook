const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");

const sourcePath = path.join(__dirname, "block_styling.js");
const source = fs.readFileSync(sourcePath, "utf8");
const purePrefix = source.split("/**")[0].replace(/^import .*$/gm, "");
const context = {};

vm.runInNewContext(
  `${purePrefix}\nglobalThis.atxHeadingLevel = atxHeadingLevel;`,
  context,
  { filename: sourcePath }
);

const { atxHeadingLevel } = context;
assert.equal(typeof atxHeadingLevel, "function");

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

console.log("block-styling-heading-rules: ok");
