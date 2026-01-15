/**
 * 查找 Mermaid 块范围
 * 
 * 遵循优先级：
 * 1. Fenced Code (```) - 如果 info string 是 "mermaid"，则标记为 MERMAID
 */
export function findMermaidRanges(doc) {
  const ranges = [];
  let i = 0;
  const len = doc.length;
  
  while (i < len) {
    // Check for Code Block Start
    if (doc[i] === '`' && doc.startsWith('```', i)) {
      const start = i;
      i += 3;
      
      // Parse info string
      let infoStart = i;
      while (i < len && doc[i] !== '\n') {
        i++;
      }
      const infoString = doc.slice(infoStart, i).trim();
      
      // Find end of block
      const contentStart = i + 1; // Skip newline
      const endMatch = doc.indexOf('```', i);
      
      if (endMatch !== -1) {
        if (infoString === "mermaid") {
             ranges.push({
                from: start,
                to: endMatch + 3,
                contentFrom: contentStart,
                contentTo: endMatch
             });
        }
        i = endMatch + 3;
      } else {
        i = len; // Unclosed
      }
      continue;
    }
    i++;
  }
  return ranges;
}
