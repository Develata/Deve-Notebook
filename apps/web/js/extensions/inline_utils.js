/**
 * Inline Tokenizer Helpers (行内解析辅助函数)
 * 提取核心逻辑以保持渲染器简洁。
 */

/**
 * 提交缓冲区文本到容器 (Flush Text Buffer)
 * @param {string} content - 完整文本
 * @param {number} lastIndex - 上次处理的索引
 * @param {number} end - 当前结束索引
 * @param {HTMLElement} container - DOM 容器
 */
export function flushText(content, lastIndex, end, container) {
    if (end > lastIndex) {
        const text = content.slice(lastIndex, end);
        if (text) container.appendChild(document.createTextNode(text));
    }
}

/**
 * 匹配反引号闭合位置 (Find Matching Backtick Closer)
 * @param {string} content - 完整文本
 * @param {number} start - 起始索引
 * @param {number} len - 文本总长
 * @returns {number} - 闭合位置索引，未找到返回 -1
 */
export function findBacktickClose(content, start, len) {
    let runLength = 0;
    while (start + runLength < len && content[start + runLength] === '`') {
        runLength++;
    }

    let j = start + runLength;
    while (j < len) {
        if (content[j] === '`') {
            let closeRun = 0;
            while (j + closeRun < len && content[j + closeRun] === '`') {
                closeRun++;
            }
            if (closeRun === runLength) {
                return j; // Found start of closer
            } else {
                j += closeRun;
            }
        } else {
            j++;
        }
    }
    return -1;
}

/**
 * 匹配公式闭合位置 (Find Math Closer)
 * @param {string} content - 完整文本
 * @param {number} start - 起始索引 ($ 之后)
 * @param {number} len - 文本总长
 * @returns {number} - 闭合位置索引，未找到返回 -1
 */
export function findMathClose(content, start, len) {
    let j = start;
    while (j < len) {
        if (content[j] === '\\') { 
            j += 2; 
            continue; 
        }
        if (content[j] === '$' && !/\s/.test(content[j - 1])) {
            return j;
        }
        j++;
    }
    return -1;
}

/**
 * 匹配样式闭合位置 (Find Style Closer)
 * @param {string} content - 完整文本
 * @param {number} start - 起始索引 (标记之后)
 * @param {string} marker - 结束标记 (e.g. "**", "*", "~~")
 * @returns {number} - 闭合位置索引，未找到返回 -1
 */
export function findStyleClose(content, start, marker) {
    const close = content.indexOf(marker, start);
    if (close !== -1) {
        // 对于斜体 *, 确保后面不是 *, 防止匹配 ** 的一部分
        if (marker === '*' && content[close + 1] === '*') {
            return -1; 
        }
        return close;
    }
    return -1;
}
