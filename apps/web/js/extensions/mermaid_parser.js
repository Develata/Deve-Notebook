import { scanFencedRanges } from "./fenced_ranges.js";

/**
 * 查找 Mermaid 块范围
 * 
 * 共享 scanner 先识别 fenced code；这里只投影 closed + exact `mermaid` info string。
 */
export function findMermaidRanges(doc, fencedRanges = scanFencedRanges(doc)) {
  return fencedRanges
    .filter((range) => range.closed && range.infoString === "mermaid")
    .map((range) => ({
      type: "BLOCK",
      from: range.from,
      to: range.to,
      contentFrom: range.contentFrom,
      contentTo: range.contentTo,
    }));
}
