/**
 * List Marker Plugin (列表标记插件)
 * 
 * 将 Markdown 列表标记 (*, -, +) 替换为视觉圆点符号 ○
 * 使用 Decoration.replace 和 Widget 实现。
 */

import { ViewPlugin, Decoration, WidgetType } from "@codemirror/view";
import { syntaxTree } from "@codemirror/language";

/**
 * 列表圆点 Widget
 */
class ListBulletWidget extends WidgetType {
    constructor(isOrdered, number) {
        super();
        this.isOrdered = isOrdered;
        this.number = number;
    }
    
    toDOM() {
        const span = document.createElement("span");
        span.className = "cm-list-bullet";
        
        if (this.isOrdered) {
            // 有序列表显示数字
            span.textContent = `${this.number}.`;
            span.className += " cm-list-ordered";
        } else {
            // 无序列表显示圆点
            span.textContent = "○";
        }
        
        return span;
    }
    
    ignoreEvent() {
        return false;
    }
}

/**
 * List Marker Plugin
 * 遍历语法树，找到 ListMark 节点并替换为 Widget
 */
export const listMarkerPlugin = ViewPlugin.fromClass(
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
            const selection = view.state.selection.main;
            
            // 辅助函数: 检查光标是否在范围内
            const isCursorIn = (nodeFrom, nodeTo) =>
                selection.head >= nodeFrom && selection.head <= nodeTo;
            
            try {
                const tree = syntaxTree(view.state);
                
                tree.iterate({
                    from,
                    to,
                    enter: (node) => {
                        // 检测 ListMark (*, -, +, 1.)
                        if (node.name === "ListMark") {
                            const parent = node.node.parent;
                            
                            // 当光标在列表项内时，显示原始标记
                            if (parent && isCursorIn(parent.from, parent.to)) {
                                return;
                            }
                            
                            // 获取标记文本判断类型
                            const markText = view.state.doc.sliceString(node.from, node.to);
                            const isOrdered = /^\d+\.$/.test(markText);
                            const number = isOrdered ? parseInt(markText) : 0;
                            
                            // 创建替换 Widget
                            const widget = Decoration.replace({
                                widget: new ListBulletWidget(isOrdered, number),
                            });
                            
                            widgets.push(widget.range(node.from, node.to));
                        }
                    },
                });
            } catch (e) {
                console.warn("ListMarkerPlugin Error:", e);
            }
            
            return Decoration.set(widgets.sort((a, b) => a.from - b.from));
        }
    },
    { decorations: (v) => v.decorations }
);
