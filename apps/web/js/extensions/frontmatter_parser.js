/**
 * 查找并验证 Frontmatter 范围 (Strict Syntax Check)
 * 
 * 逻辑:
 * 1. 必须以 --- 开头 (第一行)
 * 2. 逐行扫描，直到遇到第二个 ---
 * 3. 实时语法检查: 中间的内容必须符合 "Key: Value" 或 YAML 结构。
 *    - 如果遇到不符合语法的行 (如普通文本且不含冒号)，则视为无效，立即abort。
 *    - 允许空行和注释 (#)。
 * 
 * @param {string} doc - 文档全文
 * @returns {object|null} - 返回 { from, to, contentFrom, contentTo } 或 null
 */
export function findFrontmatterRange(doc) {
  // 1. Check Start
  if (!doc.startsWith("---")) return null;
  
  // Start scanning from line 1
  // We need precise positioning
  let pos = 3; // After first ---
  
  // Check if first --- is followed by newline
  if (pos < doc.length && doc[pos] === '\r') pos++; 
  if (pos < doc.length && doc[pos] === '\n') pos++;
  else {
      // If doc is "---abc", it's not valid FM start usually (needs newline)
      if (pos < doc.length) return null; 
  }
  
  let startContent = pos;
  
  // Find next line
  while (pos < doc.length) {
     let nextNewline = doc.indexOf('\n', pos);
     if (nextNewline === -1) nextNewline = doc.length;
     
     // Get line content (excluding newline)
     let lineContent = doc.slice(pos, nextNewline);
     if (lineContent.endsWith('\r')) lineContent = lineContent.slice(0, -1);
     
     // 2. Check for End Delimiter
     if (lineContent === "---") {
         // Found end!
         // Return range including the closing ---
         // Next line starts at nextNewline + 1
         return {
             from: 0,
             to: nextNewline, 
             contentFrom: startContent,
             contentTo: pos
         };
     }
     
     // 3. Syntax Validation. Rules:
     // - Empty line: OK
     // - Comment (#): OK
     // - List item (- ): OK
     // - Key-Value (anything: anything): OK
     // - Invalid: Plain text without colon or dash?
     
     const trimmed = lineContent.trim();
     if (trimmed.length > 0) {
         if (trimmed.startsWith("#")) {
             // Comment -> OK
         } else if (trimmed.startsWith("- ")) {
             // List -> OK
         } else if (trimmed.includes(":")) {
             // Key Value -> OK (Simple check)
         } else {
             // Plain text found? -> Abort! 
             return null; 
         }
     }
     
     // Move to next line
     pos = nextNewline + 1;
  }
  
  return null; // Closed --- not found
}
