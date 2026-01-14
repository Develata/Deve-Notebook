/**
 * 查找文档中的数学公式范围
 * 
 * 匹配 $...$ (行内) 和 $$...$$ (块级)。
 * 忽略转义的美元符号 (\$)。
 * 
 * @param {string} docString - 文档全文
 * @returns {Array} - 返回范围对象数组 [{ type, from, to, contentFrom, contentTo }]
 */
export function findMathRanges(docString) {
  const mathRanges = [];
  const regexAnyDollar = /\$+/g;
  
  // 检查是否转义 (前面有奇数个反斜杠)
  const isEscaped = (index) => {
    let backslashes = 0;
    let i = index - 1;
    while (i >= 0 && docString[i] === "\\") {
      backslashes++;
      i--;
    }
    return backslashes % 2 === 1;
  };

  let match;
  let tokens = [];

  // 第一步：收集所有可能的定界符 token
  while ((match = regexAnyDollar.exec(docString)) !== null) {
    const val = match[0];
    const index = match.index;

    if (isEscaped(index)) continue;

    if (val === "$$" || val === "$") {
      tokens.push({ type: val, index });
    }
  }

  let mode = "NONE";
  let startToken = null;

  // 第二步：配对 token
  for (let i = 0; i < tokens.length; i++) {
    let t = tokens[i];

    if (mode === "NONE") {
      if (t.type === "$$") {
        mode = "BLOCK";
        startToken = t;
      } else if (t.type === "$") {
        mode = "INLINE";
        startToken = t;
      }
    } else if (mode === "BLOCK") {
      if (t.type === "$$") {
        // 找到块级结束
        mathRanges.push({ 
            type: "BLOCK",
            from: startToken.index, 
            to: t.index + 2,
            contentFrom: startToken.index + 2,
            contentTo: t.index
        });
        mode = "NONE";
        startToken = null;
      }
    } else if (mode === "INLINE") {
       // 行内公式不能包含双换行 (段落分隔)
       let contentSoFar = docString.slice(startToken.index + 1, t.index);
       if (contentSoFar.includes("\n\n")) {
         mode = "NONE";
         startToken = null;
         i--; // 回退，重新评估当前 token
         continue;
       }

       if (t.type === "$") {
         // 找到行内结束
         mathRanges.push({
             type: "INLINE",
             from: startToken.index,
             to: t.index + 1,
             contentFrom: startToken.index + 1,
             contentTo: t.index
         });
         mode = "NONE";
         startToken = null;
       } else if (t.type === "$$") {
         // 在行内模式遇到 $$，视为错误或重置，重新开始
         mode = "NONE";
         startToken = null;
         i--;
         continue;
       }
    }
  }

  return mathRanges;
}
