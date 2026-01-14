/**
 * 查找文档中的数学公式范围
 * 
 * 匹配 $...$ (行内) 和 $$...$$ (块级)。
 * 忽略转义的美元符号 (\$)。
 * 
 * @param {string} docString - 文档全文
 * @returns {Array} - 返回范围对象数组 [{ type, from, to, contentFrom, contentTo }]
 */
/**
 * 查找文档中的数学公式范围 (Robust GFM-aware Parser)
 * 
 * 遵循以下优先级 (Priority):
 * 1. Fenced Code (```) & Inline Code (`) -> 忽略内部内容
 * 2. Escaping (\) -> 忽略转义字符
 * 3. Math ($$) -> Block
 * 4. Math ($) -> Inline (Smart Boundary Check)
 * 
 * @param {string} doc - 文档全文
 * @returns {Array} - 返回范围对象数组
 */
export function findMathRanges(doc) {
  const ranges = [];
  let i = 0;
  const len = doc.length;
  
  while (i < len) {
    const char = doc[i];
    
    // 1. Escaping: 跳过转义字符 (例如 \$)
    if (char === '\\') {
      i += 2; 
      continue;
    }
    
    // 2. Fenced Code Block: ```
    if (char === '`' && doc.startsWith('```', i)) {
      // 查找代码块结束
      const start = i;
      i += 3;
      const endMatch = doc.indexOf('```', i);
      if (endMatch !== -1) {
        i = endMatch + 3;
      } else {
        i = len; // 未闭合，跳到末尾
      }
      continue; 
    }
    
    // 3. Inline Code: `
    if (char === '`') {
      const start = i;
      // 计算起始反引号数量 (Run Length)
      let runLength = 0;
      while (i + runLength < len && doc[i + runLength] === '`') {
        runLength++;
      }
      
      i += runLength;
      
      // 查找匹配的结束反引号串
      let closed = false;
      while (i < len) {
        if (doc[i] === '`') {
           // 检查是否是一串相同长度的反引号
           let closeRun = 0;
           while (i + closeRun < len && doc[i + closeRun] === '`') {
             closeRun++;
           }
           
           if (closeRun === runLength) {
             i += closeRun;
             closed = true;
             break;
           } else {
             // 长度不匹配，继续向前
             i += closeRun; 
           }
        } else {
           i++;
        }
      }
      continue; // 无论是否闭合，都跳过已扫描部分
    }
    
    // 4. Block Math: $$
    // (注意: 必须在 check $ 之前)
    if (char === '$' && doc.startsWith('$$', i)) {
       const start = i;
       i += 2; 
       const endMatch = doc.indexOf('$$', i);
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
    
    // 5. Inline Math: $
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
