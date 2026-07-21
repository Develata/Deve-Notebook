import { scanFencedRanges } from "./fenced_ranges.js";

/**
 * 查找文档中的数学公式范围 (Robust GFM-aware Parser)
 * 
 * 遵循以下优先级 (Priority):
 * 1. Fenced Code & Inline Code (`) -> 忽略内部内容
 * 2. Escaping (\) -> 忽略转义字符
 * 3. Math ($$) -> Block
 * 4. Math ($) -> Inline (Smart Boundary Check)
 * 
 * @param {string} doc - 文档全文
 * @param {Array} fencedRanges - 共享 fenced scanner 的 half-open ranges
 * @returns {Array} - 返回范围对象数组
 */
export function findMathRanges(doc, fencedRanges = scanFencedRanges(doc)) {
  const ranges = [];
  const backtickRuns = indexBacktickRuns(doc);
  let i = 0;
  const len = doc.length;
  let fenceIndex = 0;
  
  while (i < len) {
    while (fenceIndex < fencedRanges.length && fencedRanges[fenceIndex].to <= i) {
      fenceIndex++;
    }
    const fence = fencedRanges[fenceIndex];
    if (fence && i >= fence.from && i < fence.to) {
      i = fence.to;
      continue;
    }

    const char = doc[i];
    
    // 1. Escaping: 跳过转义字符 (例如 \$)
    if (char === '\\') {
      i += 2; 
      continue;
    }
    
    // 2. Inline Code: `
    if (char === '`') {
      const run = backtickRuns.get(i);
      const close = run?.next;
      if (close && (!fence || close.from < fence.from)) i = close.to;
      else i = run?.to ?? i + 1;
      continue;
    }
    
    // 3. Block Math: $$
    // (注意: 必须在 check $ 之前)
    if (char === '$' && doc.startsWith('$$', i)) {
       const start = i;
       i += 2; 
       const nextFence = fencedRanges[fenceIndex];
       const endMatch = findBlockMathEnd(doc, i, nextFence?.from ?? len, backtickRuns);
       if (endMatch !== -1) {
          ranges.push({
             type: "BLOCK",
             from: start,
             to: endMatch + 2,
             contentFrom: start + 2,
             contentTo: endMatch
          });
          i = endMatch + 2;
       } else {
          // 未闭合，当作普通文本
          i += 1;
       }
       continue;
    }
    
    // 4. Inline Math: $
    if (char === '$') {
       // Smart Boundary Check (Start)
       // 规则: $ 紧邻非空字符 (First char non-whitespace)
       const nextChar = doc[i+1];
       if (!nextChar || /\s/.test(nextChar)) {
          i++; // 无效起始，跳过
          continue; 
       }
       
       const start = i;
       i++; // 进入内容
       
       // 扫描结束符
       let closeFound = -1;
       let scanI = i;
       
       while (scanI < len) {
          const nextFence = fencedRanges[fenceIndex];
          if (nextFence && scanI >= nextFence.from) break;
          const c = doc[scanI];
          
          if (c === '\\') {
             scanI += 2;
             continue;
          }
          
          if (c === '$') {
             // 检查是否是 $$ (如果是 $$，说明不是行内公式结束，甚至可能是空行内公式 $$，但通常 $$ 优先被 Parse Block 捕获)
             // 细则: 如果 Math Parser 遇到 $$，通常视为行内结束吗？或者 Block？
             // 这里简化: 如果遇到 $，检查 Boundary
             
             // Smart Boundary Check (End)
             // 规则: $ 前紧邻非空字符 (Last char non-whitespace)
             const prevChar = doc[scanI - 1];
             if (!/\s/.test(prevChar)) {
                 closeFound = scanI;
                 break;
             }
          }
          
          // 额外安全机制: 行内公式不能包含空行 (Blank Line)
          if (c === '\n' && doc[scanI + 1] === '\n') {
             break; // Abort
          }
          
          scanI++;
       }
       
       if (closeFound !== -1) {
          ranges.push({
             type: "INLINE",
             from: start,
             to: closeFound + 1,
             contentFrom: start + 1,
             contentTo: closeFound
          });
          i = closeFound + 1;
       } else {
          i++; // 未找到闭合
       }
       continue;
    }
    
    i++;
  }
  
  return ranges;
}

function findBlockMathEnd(doc, from, limit, backtickRuns) {
  let i = from;
  while (i < limit) {
    if (doc[i] === '\\') {
      i += 2;
      continue;
    }
    if (doc[i] === '`') {
      const run = backtickRuns.get(i);
      const close = run?.next;
      i = close && close.from < limit ? close.to : (run?.to ?? i + 1);
      continue;
    }
    if (doc.startsWith('$$', i)) return i;
    i++;
  }
  return -1;
}

function indexBacktickRuns(doc) {
  const runs = [];
  for (let i = 0; i < doc.length;) {
    if (doc[i] !== '`') {
      i++;
      continue;
    }
    const from = i;
    while (i < doc.length && doc[i] === '`') i++;
    runs.push({ from, to: i, length: i - from, next: null });
  }

  const nextByLength = new Map();
  const runsByStart = new Map();
  for (let i = runs.length - 1; i >= 0; i--) {
    const run = runs[i];
    run.next = nextByLength.get(run.length) || null;
    const escapedPrefix = precedingBackslashCount(doc, run.from) % 2 === 1;
    if (escapedPrefix && run.length > 1) {
      const suffixLength = run.length - 1;
      runsByStart.set(run.from + 1, {
        from: run.from + 1,
        to: run.to,
        length: suffixLength,
        next: nextByLength.get(suffixLength) || null,
      });
    }
    nextByLength.set(run.length, run);
    runsByStart.set(run.from, run);
  }
  return runsByStart;
}

function precedingBackslashCount(doc, position) {
  let count = 0;
  for (let i = position - 1; i >= 0 && doc[i] === '\\'; i--) count++;
  return count;
}
