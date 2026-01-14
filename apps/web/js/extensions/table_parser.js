/**
 * Table Parser (表格解析器)
 * 
 * 负责解析纯文本的 Markdown 表格语法，不包含任何 DOM 或 CodeMirror 依赖。
 */

/**
 * 解析表格文本
 * 
 * @param {string} tableText - 表格 Markdown 文本
 * @returns {Object|null} - 解析后的数据结构 { header, alignments, body } 或 null
 */
export function parseTable(tableText) {
    const lines = tableText.trim().split('\n');
    if (lines.length < 2) return null;
    
    // 辅助函数：解析一行，移除首尾竖线并分割
    const parseRow = (line) => {
        return line
            .replace(/^\|/, '')
            .replace(/\|$/, '')
            .split('|')
            .map(cell => cell.trim());
    };
    
    const headerRow = parseRow(lines[0]);
    const separatorLine = lines[1];
    
    // 校验分隔行 - 必须包含 | 和 -
    // 例如: |---|---|, |-|-|, | --- | --- |
    if (!separatorLine.includes('|') || !separatorLine.includes('-')) return null;
    
    // 分隔行每个单元格只能包含 -, :, 和空格
    const separatorCells = parseRow(separatorLine);
    const validSeparator = separatorCells.every(cell => /^:?-+:?$/.test(cell.trim()) || cell.trim() === '');
    if (!validSeparator) return null;
    
    // 解析对齐方式
    const alignments = separatorCells.map(sep => {
        sep = sep.trim();
        const left = sep.startsWith(':');
        const right = sep.endsWith(':');
        if (left && right) return 'center';
        if (right) return 'right';
        return 'left';
    });
    
    // 解析主体行
    const bodyRows = lines.slice(2).map(parseRow);
    
    return { header: headerRow, alignments, body: bodyRows };
}

/**
 * 在文档中查找表格及其范围
 * 
 * @param {string} doc - 文档全文
 * @returns {Array} - 表格范围数组 [{ from, to, data }]
 */
export function findTableRanges(doc) {
    const ranges = [];
    const lines = doc.split('\n');
    let i = 0;
    let pos = 0;
    
    while (i < lines.length) {
        const line = lines[i];
        
        // 检查当前行是否可能是表头 (包含 | )
        if (line.includes('|') && i + 1 < lines.length) {
            const nextLine = lines[i + 1];
            
            // 检查下一行是否是分隔符 (包含 | 和 -)
            if (nextLine.includes('|') && nextLine.includes('-')) {
                const startPos = pos;
                let tableEnd = i + 1;
                
                // 向下查找表格结束位置
                for (let j = i + 2; j < lines.length; j++) {
                    if (lines[j].includes('|')) {
                        tableEnd = j;
                    } else {
                        break;
                    }
                }
                
                // 计算结束时的字符位置 (累加行长 + 换行符)
                let endPos = startPos;
                for (let j = i; j <= tableEnd; j++) {
                    endPos += lines[j].length + 1;
                }
                endPos--; // 移除最后一个换行符带来的偏移
                
                // 提取表格文本并解析
                const tableText = lines.slice(i, tableEnd + 1).join('\n');
                const tableData = parseTable(tableText);
                
                if (tableData) {
                    ranges.push({
                        from: startPos,
                        to: endPos,
                        data: tableData
                    });
                }
                
                // 更新遍历索引，跳过已处理的行
                i = tableEnd + 1;
                pos = endPos + 1;
                continue;
            }
        }
        
        pos += line.length + 1;
        i++;
    }
    
    return ranges;
}
