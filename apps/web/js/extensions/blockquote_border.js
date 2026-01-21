/**
 * Blockquote Border Plugin (引用块边框插件)
 * 
 * 为引用块添加递进式左边框，嵌套越深边框越明显。
 * 同时隐藏 > 符号（当光标不在该行时）。
 */

import { ViewPlugin, Decoration } from "@codemirror/view";

/**
 * 计算行的引用嵌套深度，同时返回 > 字符的位置
 * @param {string} lineText - 行文本
 * @returns {{ depth: number, quotePositions: number[] }}
 */
function analyzeQuoteLine(lineText) {
    let depth = 0;
    const quotePositions = []; // 记录每个 > 的相对位置
    
    for (let i = 0; i < lineText.length; i++) {
        const char = lineText[i];
        if (char === '>') {
            depth++;
            quotePositions.push(i);
        } else if (char !== ' ' && char !== '\t') {
            break;
        }
    }
    return { depth, quotePositions };
}

/**
 * Blockquote Border Plugin
 * 为引用块行添加基于嵌套深度的 CSS 类，并隐藏 > 符号
 */
export const blockquoteBorderPlugin = ViewPlugin.fromClass(
    class {
        constructor(view) {
            this.decorations = this.computeDecorations(view);
        }
        
        update(update) {
            if (update.docChanged || update.viewportChanged || update.selectionSet) {
                this.decorations = this.computeDecorations(update.view);
            }
        }
        
        computeDecorations(view) {
            const widgets = [];
            const { from, to } = view.viewport;
            const doc = view.state.doc;
            const selection = view.state.selection.main;
            
            // 辅助函数: 检查光标是否在行内
            const isCursorOnLine = (lineFrom, lineTo) =>
                selection.head >= lineFrom && selection.head <= lineTo;
            
            // 遍历视口内的行
            let startLine = doc.lineAt(from).number;
            let endLine = doc.lineAt(to).number;
            
            for (let i = startLine; i <= endLine; i++) {
                const line = doc.line(i);
                const lineText = line.text;
                const { depth, quotePositions } = analyzeQuoteLine(lineText);
                
                if (depth > 0) {
                    // 添加深度样式 (Line Decoration)
                    const effectiveDepth = Math.min(depth, 5);
                    const className = `cm-blockquote-depth-${effectiveDepth}`;
                    widgets.push(
                        Decoration.line({ class: className }).range(line.from)
                    );
                    
                    // 隐藏 > 符号 (Replace Decoration)
                    // 只有当光标不在该行时才隐藏
                    if (!isCursorOnLine(line.from, line.to)) {
                        for (const pos of quotePositions) {
                            const absPos = line.from + pos;
                            // 使用 replace 完全隐藏 > 字符
                            widgets.push(
                                Decoration.replace({
                                    block: false,  // > 是行内字符，不是块级
                                    inclusive: false
                                }).range(absPos, absPos + 1)
                            );
                        }
                    }
                }
            }
            
            return Decoration.set(widgets.sort((a, b) => a.from - b.from));
        }
    },
    { decorations: (v) => v.decorations }
);
