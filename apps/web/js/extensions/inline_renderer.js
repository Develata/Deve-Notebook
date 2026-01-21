import { findMathRanges } from "./utils.js";

/**
 * HTML 转义工具
 */
function escapeHtml(text) {
    return text.replace(/&/g, "&amp;")
               .replace(/</g, "&lt;")
               .replace(/>/g, "&gt;");
}

/**
 * 渲染普通 Markdown 文本
 * 优先级: Inline Code > Styles (Bold, Italic, Strike)
 */
function appendMarkdownText(text, container) {
    // 1. Split by Inline Code (`...`)
    // 使用捕获组保留分隔符部分
    const parts = text.split(/(`[^`]+`)/g);

    parts.forEach(part => {
        if (part.startsWith('`') && part.endsWith('`') && part.length > 2) {
            // --- Inline Code ---
            const code = part.slice(1, -1);
            const span = document.createElement("span");
            span.className = "cm-inline-code"; // 使用 CodeMirror 样式类
            span.textContent = code; // textContent 自动处理 HTML 转义
            container.appendChild(span);
        } else {
            // --- Styles (Bold, Italic, Strike) ---
            if (!part) return;
            
            let html = escapeHtml(part);
            
            // Bold (**...**)
            html = html.replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>');
            
            // Italic (*...*)
            html = html.replace(/\*([^*]+)\*/g, '<em>$1</em>');
            
            // Strikethrough (~~...~~)
            html = html.replace(/~~([^~]+)~~/g, '<del>$1</del>');
            
            // [TODO] Links support: [text](url)
            // 需要更复杂的解析以防止 URL 中的字符破坏 HTML
            
            const span = document.createElement("span");
            span.innerHTML = html;
            container.appendChild(span);
        }
    });
}

/**
 * 渲染行内内容
 * 优先级: Math > Code > Styles
 * @param {string} content - Markdown 文本内容
 * @param {HTMLElement} container - 目标容器
 */
export function renderInline(content, container) {
    if (!content) return;
    
    // 1. Math Parsing (优先识别公式范围)
    // findMathRanges 内部逻辑已跳过 Code Block/Inline Code
    const mathRanges = findMathRanges(content);
    let lastIndex = 0;
    
    mathRanges.forEach(range => {
        // 先渲染公式前的普通文本 (含 Inline Code)
        if (range.from > lastIndex) {
            appendMarkdownText(content.slice(lastIndex, range.from), container);
        }
        
        // 渲染 Math Widget
        const mathContent = content.slice(range.contentFrom, range.contentTo);
        const mathSpan = document.createElement("span");
        mathSpan.className = "cm-math-widget" + (range.type === "BLOCK" ? " cm-block-math" : "");
        
        try {
            if (window.katex) {
                window.katex.render(mathContent, mathSpan, {
                    throwOnError: false,
                    displayMode: range.type === "BLOCK"
                });
            } else {
                mathSpan.textContent = (range.type === "BLOCK" ? "$$" : "$") + mathContent + (range.type === "BLOCK" ? "$$" : "$");
            }
        } catch (e) {
            mathSpan.textContent = "Error";
        }
        
        container.appendChild(mathSpan);
        lastIndex = range.to;
    });
    
    // 渲染剩余文本
    if (lastIndex < content.length) {
        appendMarkdownText(content.slice(lastIndex), container);
    }
}
