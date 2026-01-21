import { findMathRanges } from "./utils.js";
import { flushText, findBacktickClose, findMathClose, findStyleClose } from "./inline_utils.js";

/**
 * 渲染行内内容 - 模块化重构版 (Modularized Renderer)
 * 优先级: Escaping > Code > Math > Styles
 * 
 * @param {string} content - Markdown 文本内容
 * @param {HTMLElement} container - 目标容器
 */
export function renderInline(content, container) {
    if (!content) return;

    let i = 0;
    let len = content.length;
    let lastIndex = 0;

    // 绑定当前上下文的 flushText
    const flush = (end) => flushText(content, lastIndex, end, container);

    while (i < len) {
        const char = content[i];

        // 1. Escaping (\)
        if (char === '\\' && i + 1 < len) {
            flush(i);
            container.appendChild(document.createTextNode(content[i + 1]));
            i += 2;
            lastIndex = i;
            continue;
        }

        // 2. Inline Code (`)
        if (char === '`') {
            const closeIndex = findBacktickClose(content, i, len);
            if (closeIndex !== -1) {
                // 计算反引号长度
                let runLength = 0;
                while (i + runLength < len && content[i + runLength] === '`') runLength++;
                
                flush(i);
                
                const codeContent = content.slice(i + runLength, closeIndex);
                const span = document.createElement("span");
                span.className = "cm-inline-code";
                span.textContent = codeContent;
                container.appendChild(span);

                i = closeIndex + runLength;
                lastIndex = i;
                continue;
            }
        }

        // 3. Inline Math ($)
        if (char === '$' && i + 1 < len && !/\s/.test(content[i+1])) {
             const closeIndex = findMathClose(content, i + 1, len);
             if (closeIndex !== -1) {
                 flush(i);
                 const mathContent = content.slice(i + 1, closeIndex);
                 const span = document.createElement("span");
                 span.className = "cm-math-widget";
                 try {
                    if (window.katex) {
                        window.katex.render(mathContent, span, { throwOnError: false, displayMode: false });
                    } else {
                        span.textContent = `$${mathContent}$`;
                    }
                 } catch(e) { span.textContent = "Math Error"; }
                 
                 container.appendChild(span);
                 i = closeIndex + 1;
                 lastIndex = i;
                 continue;
             }
        }

        // 4. Styles (Bold **, Italic *, Strike ~~)
        if (char === '*' || char === '~') {
            const isBold = char === '*' && content[i+1] === '*';
            const isStrike = char === '~' && content[i+1] === '~';
            const marker = isBold ? '**' : (isStrike ? '~~' : '*');
            
            // 提交之前的文本
            // 注意: 这里不预先 check 闭合，为了性能? 或者应该 check?
            // 原逻辑是先找 close，找不到就不处理。保持一致。
            
            const startContent = i + marker.length;
            const closeIndex = findStyleClose(content, startContent, marker);
            
            if (closeIndex !== -1) {
                flush(i);
                const inner = content.slice(startContent, closeIndex);
                const tag = isBold ? "strong" : (isStrike ? "del" : "em");
                const el = document.createElement(tag);
                
                renderInline(inner, el); // Recursion
                container.appendChild(el);
                
                i = closeIndex + marker.length;
                lastIndex = i;
                continue;
            }
        }

        i++;
    }

    flush(len);
}
