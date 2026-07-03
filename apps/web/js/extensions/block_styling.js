import { ViewPlugin, Decoration } from "@codemirror/view";
import { syntaxTree } from "@codemirror/language";
import { findMathRanges, findFrontmatterRange } from "./utils.js";

const ATX_HEADING_LINE_RE = /^ {0,3}(#{1,6})(?:[ \t]|$)/;
// Editing affordance only: keep active CJK "#标题" candidates tall without changing saved Markdown semantics.
const ACTIVE_CJK_ATX_HEADING_LINE_RE = /^ {0,3}(#{1,6})(?=[^\s#\x00-\x7F])/u;

function atxHeadingMatch(lineText, isActiveLine) {
  const atxHeading = lineText.match(ATX_HEADING_LINE_RE);
  if (atxHeading) {
    return {
      level: atxHeading[1].length,
      openerOffset: atxHeading[0].indexOf(atxHeading[1]),
    };
  }

  const activeCjkCandidate = isActiveLine
    ? lineText.match(ACTIVE_CJK_ATX_HEADING_LINE_RE)
    : null;
  if (activeCjkCandidate) {
    return {
      level: activeCjkCandidate[1].length,
      openerOffset: activeCjkCandidate[0].indexOf(activeCjkCandidate[1]),
    };
  }

  return null;
}

function atxHeadingLevel(lineText, isActiveLine) {
  return atxHeadingMatch(lineText, isActiveLine)?.level ?? null;
}

function headingLineClass(headingLevel) {
  const baseClass = `cm-h${headingLevel}`;
  if (headingLevel <= 3) {
    return `${baseClass} cm-heading-line cm-heading-line-${headingLevel}`;
  }
  return baseClass;
}

function rangeContainsPosition(ranges, pos) {
  return ranges.some((range) => pos >= range.from && pos < range.to);
}

function protectedHeadingRanges(docText) {
  const ranges = findMathRanges(docText).map(({ from, to }) => ({ from, to }));
  const frontmatter = findFrontmatterRange(docText);
  if (frontmatter) {
    ranges.push({ from: frontmatter.from, to: frontmatter.to });
  }
  return ranges;
}

function textMayContainProtectedDelimiter(text) {
  return (
    text.includes("$") ||
    text.includes("`") ||
    text.includes("\\") ||
    text.includes("---") ||
    text.includes("\n")
  );
}

function mapProtectedRanges(ranges, changes) {
  return ranges
    .map(({ from, to }) => ({
      from: changes.mapPos(from, 1),
      to: changes.mapPos(to, -1),
    }))
    .filter(({ from, to }) => to > from);
}

function frontmatterLineCanContinue(lineContent) {
  const trimmed = lineContent.endsWith("\r")
    ? lineContent.slice(0, -1).trim()
    : lineContent.trim();
  return (
    trimmed.length === 0 ||
    trimmed.startsWith("#") ||
    trimmed.startsWith("- ") ||
    trimmed.includes(":")
  );
}

function docStartsWithFrontmatterFence(doc) {
  const prefix = doc.sliceString(0, Math.min(doc.length, 4));
  return prefix === "---" || prefix === "---\n" || prefix === "---\r";
}

function frontmatterCandidateBoundary(doc) {
  if (!docStartsWithFrontmatterFence(doc)) return 0;
  if (doc.length === 3) return 3;

  for (let lineNo = 2; lineNo <= doc.lines; lineNo++) {
    const line = doc.line(lineNo);
    const lineText = line.text.endsWith("\r") ? line.text.slice(0, -1) : line.text;
    if (lineText === "---") return line.to;
    if (!frontmatterLineCanContinue(lineText)) return line.to;
  }
  return doc.length;
}

function frontmatterChangeNeedsFullRescan(update, fromA, fromB) {
  const oldBoundary = frontmatterCandidateBoundary(update.startState.doc);
  const newBoundary = frontmatterCandidateBoundary(update.view.state.doc);
  return (
    (oldBoundary > 0 && fromA <= oldBoundary) ||
    (newBoundary > 0 && fromB <= newBoundary)
  );
}

function protectedRangesNeedFullRescan(update) {
  let needsRescan = false;
  update.changes.iterChanges((fromA, toA, fromB, toB, inserted) => {
    const oldFrom = Math.max(0, fromA - 3);
    const oldTo = Math.min(update.startState.doc.length, toA + 3);
    const newFrom = Math.max(0, fromB - 3);
    const newTo = Math.min(update.view.state.doc.length, toB + 3);
    const oldText = update.startState.doc.sliceString(oldFrom, oldTo);
    const newText = update.view.state.doc.sliceString(newFrom, newTo);
    if (
      textMayContainProtectedDelimiter(oldText) ||
      textMayContainProtectedDelimiter(newText) ||
      textMayContainProtectedDelimiter(inserted.toString()) ||
      frontmatterChangeNeedsFullRescan(update, fromA, fromB)
    ) {
      needsRescan = true;
    }
  });
  return needsRescan;
}

function nextProtectedRanges(currentRanges, update) {
  if (protectedRangesNeedFullRescan(update)) {
    return protectedHeadingRanges(update.view.state.doc.toString());
  }
  return mapProtectedRanges(currentRanges, update.changes);
}

function headingLineClassForLine(line, isActiveLine, protectedRanges) {
  const heading = atxHeadingMatch(line.text, isActiveLine);
  if (!heading) return null;

  const openerPos = line.from + heading.openerOffset;
  if (rangeContainsPosition(protectedRanges, openerPos)) return null;

  return headingLineClass(heading.level);
}

/**
 * Block Styling Plugin
 * 
 * Iterates through the Markdown syntax tree and applies background classes 
 * to lines that are part of:
 * 1. Fenced Code Block (.cm-code-block-line)
 * 2. Blockquote (.cm-blockquote-line)
 * 3. ATX Headings (.cm-h1 ... .cm-h6)
 */
export const blockStyling = ViewPlugin.fromClass(
  class {
    constructor(view) {
      this.protectedRanges = protectedHeadingRanges(view.state.doc.toString());
      this.decorations = this.computeDecorations(view);
    }

    update(update) {
      if (update.docChanged) {
        this.protectedRanges = nextProtectedRanges(this.protectedRanges, update);
      }

      if (
        update.docChanged ||
        update.viewportChanged ||
        update.searchChanged ||
        update.selectionSet
      ) {
        this.decorations = this.computeDecorations(update.view);
      }
    }

    computeDecorations(view) {
      let widgets = [];
      const codeLineStarts = new Set();
      const { from, to } = view.viewport;
      const doc = view.state.doc;
      const protectedRanges = this.protectedRanges ?? [];
      const activeLineNumber = doc.lineAt(view.state.selection.main.head).number;

      // 遍历语法树
      const tree = syntaxTree(view.state);
      
      tree.iterate({
        from,
        to,
        enter: (node) => {
          // 处理 FencedCode, IndentedCode 和通用的 CodeBlock
          if (["FencedCode", "IndentedCode", "CodeBlock"].includes(node.name)) {
             let startLine = doc.lineAt(node.from);
             let endLine = doc.lineAt(node.to);
             
             for (let i = startLine.number; i <= endLine.number; i++) {
                 let line = doc.line(i);
                 let lineClasses = "cm-code-block-line";
                 
                 // 圆角逻辑
                 if (i === startLine.number) lineClasses += " cm-code-block-start";
                 if (i === endLine.number) lineClasses += " cm-code-block-end";
                 
                 codeLineStarts.add(line.from);
                 widgets.push(Decoration.line({ class: lineClasses }).range(line.from));
             }
          }
        },
      });

      const firstLine = doc.lineAt(from);
      const lastLine = doc.lineAt(to);
      for (let i = firstLine.number; i <= lastLine.number; i++) {
          const line = doc.line(i);
          if (codeLineStarts.has(line.from)) continue;
          const lineClass = headingLineClassForLine(
              line,
              i === activeLineNumber,
              protectedRanges
          );
          if (!lineClass) continue;
          widgets.push(
              Decoration.line({ class: lineClass }).range(line.from)
          );
      }

      return Decoration.set(widgets, true); 
    }
  },
  {
    decorations: (v) => v.decorations,
  }
);
