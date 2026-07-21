import { StateField } from "@codemirror/state";
import { scanFencedRanges } from "./fenced_ranges.js";
import { findMathRanges } from "./math_parser.js";
import { findMermaidRanges } from "./mermaid_parser.js";
import { findTableRanges } from "./table_parser.js";

const KINDS = ["math", "mermaid", "table"];

export class RenderRangeIndex {
    constructor(rangesByKind, fencedRanges) {
        this.rangesByKind = rangesByKind;
        this.fencedRanges = fencedRanges;
    }

    ranges(kind) {
        return this.rangesByKind[kind] || [];
    }

    query(kind, selections) {
        const ranges = this.ranges(kind);
        const found = new Map();
        for (const selection of selections) {
            const empty = selection.from === selection.to;
            let i = firstRangeEndingAfter(ranges, selection.from);
            while (
                i < ranges.length
                && (empty ? ranges[i].from <= selection.from : ranges[i].from < selection.to)
            ) {
                const range = ranges[i];
                const intersects = empty
                    ? selection.from >= range.from && selection.from < range.to
                    : selection.to > range.from && selection.from < range.to;
                if (intersects) {
                    found.set(range.key, range);
                }
                i++;
            }
        }
        return [...found.values()];
    }

    at(kind, position) {
        const matches = this.query(kind, [{ from: position, to: position }]);
        return matches.length > 0 ? matches[0] : null;
    }

    matchingFailure(payload) {
        if (!payload || !KINDS.includes(payload.kind)) return null;
        return this.ranges(payload.kind).find((range) => (
            range.from === payload.from
            && range.to === payload.to
            && range.sourceText === payload.sourceText
        )) || null;
    }
}

export function buildRenderRangeIndex(doc) {
    const fencedRanges = scanFencedRanges(doc);
    const mathRanges = findMathRanges(doc, fencedRanges).map((range) => ({
        ...range,
        kind: "math",
        sourceText: doc.slice(range.from, range.to),
        content: doc.slice(range.contentFrom, range.contentTo),
        key: renderRangeKey("math", range.from, range.to),
    }));
    const mermaidRanges = findMermaidRanges(doc, fencedRanges).map((range) => ({
        ...range,
        kind: "mermaid",
        sourceText: doc.slice(range.from, range.to),
        content: doc.slice(range.contentFrom, range.contentTo),
        key: renderRangeKey("mermaid", range.from, range.to),
    }));
    const tableProtected = mergeSortedRanges(
        fencedRanges,
        mathRanges.filter((range) => range.type === "BLOCK"),
    );
    const tableRanges = findTableRanges(doc, tableProtected).map((range) => ({
        ...range,
        kind: "table",
        sourceText: doc.slice(range.from, range.to),
        content: doc.slice(range.from, range.to),
        key: renderRangeKey("table", range.from, range.to),
    }));

    return new RenderRangeIndex({
        math: mathRanges,
        mermaid: mermaidRanges,
        table: tableRanges,
    }, fencedRanges);
}

export const renderRangeIndexField = StateField.define({
    create(state) {
        return buildRenderRangeIndex(state.doc.toString());
    },
    update(index, transaction) {
        return transaction.docChanged
            ? buildRenderRangeIndex(transaction.state.doc.toString())
            : index;
    },
});

export function renderRangeKey(kind, from, to) {
    return `${kind}:${from}:${to}`;
}

function firstRangeEndingAfter(ranges, position) {
    let low = 0;
    let high = ranges.length;
    while (low < high) {
        const mid = (low + high) >> 1;
        if (ranges[mid].to <= position) low = mid + 1;
        else high = mid;
    }
    return low;
}

function mergeSortedRanges(left, right) {
    const merged = [];
    let leftIndex = 0;
    let rightIndex = 0;
    while (leftIndex < left.length || rightIndex < right.length) {
        const leftRange = left[leftIndex];
        const rightRange = right[rightIndex];
        if (
            rightRange === undefined
            || (
                leftRange !== undefined
                && (leftRange.from < rightRange.from
                    || (leftRange.from === rightRange.from && leftRange.to <= rightRange.to))
            )
        ) {
            merged.push(leftRange);
            leftIndex++;
        } else {
            merged.push(rightRange);
            rightIndex++;
        }
    }
    return merged;
}
