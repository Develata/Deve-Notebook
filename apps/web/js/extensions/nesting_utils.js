/**
 * Nesting Utils (嵌套工具)
 * 
 * 计算 Markdown 行的嵌套深度，用于递进式缩进和样式。
 * 支持 Blockquote 和 List 的嵌套检测。
 */

/**
 * 计算行文本的嵌套信息
 * @param {string} lineText - 行文本内容
 * @returns {{ quoteDepth: number, listDepth: number, listMarker: string|null }}
 */
export function calculateNesting(lineText) {
    let quoteDepth = 0;
    let listDepth = 0;
    let listMarker = null;
    
    // 逐字符扫描，计算 > 的数量
    let i = 0;
    while (i < lineText.length) {
        const char = lineText[i];
        
        // 跳过空白
        if (char === ' ' || char === '\t') {
            i++;
            continue;
        }
        
        // 检测引用标记
        if (char === '>') {
            quoteDepth++;
            i++;
            continue;
        }
        
        // 检测列表标记 (*, -, +)
        if ((char === '*' || char === '-' || char === '+') && 
            i + 1 < lineText.length && lineText[i + 1] === ' ') {
            listMarker = char;
            break;
        }
        
        // 检测有序列表 (1., 2., etc)
        if (/\d/.test(char)) {
            let numEnd = i;
            while (numEnd < lineText.length && /\d/.test(lineText[numEnd])) {
                numEnd++;
            }
            if (lineText[numEnd] === '.' && lineText[numEnd + 1] === ' ') {
                listMarker = 'ordered';
                break;
            }
        }
        
        // 非标记字符，停止扫描
        break;
    }
    
    return { quoteDepth, listDepth, listMarker };
}

/**
 * 获取行的有效内容起始位置 (跳过 > 和空格)
 * @param {string} lineText - 行文本内容
 * @returns {number} - 内容起始索引
 */
export function getContentStart(lineText) {
    let i = 0;
    while (i < lineText.length) {
        const char = lineText[i];
        if (char === ' ' || char === '\t' || char === '>') {
            i++;
        } else {
            break;
        }
    }
    return i;
}
