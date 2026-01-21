import { WidgetType, Decoration, EditorView } from "@codemirror/view";
import { StateField } from "@codemirror/state";
import { findMermaidRanges } from "./mermaid_parser.js";
import mermaid from "mermaid"; // 静态导入 (已打包)

// --- 初始化 Mermaid (Initialize) ---
// 立即初始化，配置不自动加载，使用默认主题
mermaid.initialize({
    startOnLoad: false,
    theme: 'default',
    securityLevel: 'loose', // 允许 HTML 标签
});

/**
 * MermaidWidget (Mermaid 渲染组件)
 * 
 * 这是一个 CodeMirror Widget，负责在编辑器中渲染 Mermaid 图表。
 * 它替换了原始的代码块（Decorator.replace），但通过计算源码行数来保持占位高度。
 */
class MermaidWidget extends WidgetType {
    constructor(code) {
        super();
        this.code = code;
        // 生成唯一的 ID，用于 Mermaid 渲染
        this.id = "mermaid-" + Math.random().toString(36).substr(2, 9);
    }

    /**
     * 创建 DOM 节点 (toDOM)
     * @param {EditorView} view - 编辑器视图实例
     */
    toDOM(view) {
        const wrapper = document.createElement("div");
        // 样式类：flex居中，白底，圆角，边框，阴影
        wrapper.className = "cm-mermaid-widget flex justify-center items-center bg-white rounded-lg border border-gray-100 my-2 shadow-sm overflow-hidden";
        
        // --- 高度计算逻辑 (Height Calculation) ---
        // 根据代码行数计算高度，以匹配源代码块的高度，避免页面抖动。
        const lineCount = this.code.split('\n').length;
        // 使用默认行高 (回退值为 22px)
        const lineHeight = view.defaultLineHeight || 22; 
        // 增加一点缓冲 (例如 1 行)，避免太拥挤
        const height = (lineCount * lineHeight); 
        
        wrapper.style.height = height + "px";
        wrapper.style.minHeight = height + "px"; // 强制一致性
        
        const container = document.createElement("div");
        // [FIX] 避免与 Mermaid 内部临时元素 ID 冲突，加个后缀
        container.id = this.id + "-cont"; 
        container.style.width = "100%"; 
        container.style.height = "100%"; // 充满 wrapper
        container.className = "flex justify-center items-center"; // SVG 居中
        wrapper.appendChild(container);

        // 调度渲染检查
        this.scheduleRender(container);

        // [Fix RangeError] 手动处理点击选择
        // 避免 CodeMirror 在复杂 SVG 上调用 posAtCoords 时崩溃
        wrapper.onclick = (e) => {
            e.preventDefault();
            const pos = view.posAtDOM(wrapper);
            if (pos !== null) {
                view.dispatch({ selection: { anchor: pos } });
                view.focus();
            }
        };

        return wrapper;
    }

    /**
     * 调度渲染 (Schedule Render)
     * 使用 requestAnimationFrame 确保在 DOM 挂载后渲染
     */
    scheduleRender(container) {
        const checkDOM = () => {
             requestAnimationFrame(() => {
                 this.renderGraph(container);
             });
        };
        checkDOM();
    }

    /**
     * 执行渲染 (Render Graph)
     * 调用 Mermaid API 生成 SVG 并注入容器
     */
    renderGraph(container) {
        container.innerHTML = "";
        
        try {
            mermaid.render(this.id, this.code)
                .then(({ svg }) => {
                    container.innerHTML = svg;
                    // 确保 SVG 适应固定高度的容器
                    const svgEl = container.querySelector('svg');
                    if(svgEl) {
                        // 强制 SVG 充满由代码行数定义的容器
                        svgEl.style.width = "100%";
                        svgEl.style.height = "100%"; 
                        svgEl.style.maxWidth = "100%";
                        svgEl.style.display = "block"; 
                        
                        // 移除显式尺寸，允许缩放
                        svgEl.removeAttribute('height'); 
                        svgEl.removeAttribute('width');  
                        
                        // [CRITICAL] 确保等比缩放 (Proportional Scaling)
                        // 这允许用户通过添加换行符（增加容器高度）来“放大”图表
                        svgEl.setAttribute('preserveAspectRatio', 'xMidYMid meet');
                    }
                })
                .catch((e) => {
                    console.error("Mermaid Render Error", e);
                    this.showError(container, e);
                });
        } catch (e) {
             console.error("Mermaid Sync Error", e);
             this.showError(container, e);
        }
    }

    /**
     * 显示错误信息 (Show Error)
     */
    showError(container, error) {
        container.innerText = "Mermaid Error:\n" + error.message;
        container.className = "text-red-500 font-mono text-xs p-2 bg-red-50 rounded";
        const pre = document.createElement("pre");
        pre.className = "mt-2 text-gray-400 whitespace-pre-wrap";
        pre.innerText = this.code;
        container.appendChild(pre);
    }

    // 忽略 CodeMirror 的 DOM 事件处理
    ignoreEvent() { return true; }
}

/**
 * 计算装饰器 (Compute Decorations)
 * 扫描文档，找到 Mermaid 代码块，并用 Widget 替换它们
 */
function computeMermaidDecorations(state) {
    let widgets = [];
    let doc = state.doc.toString();
    const ranges = findMermaidRanges(doc);
    const selection = state.selection.main; 

    for (let r of ranges) {
        // [Interaction Fix] 交互修复
        // 检查选择范围是否与代码块重叠
        // 如果光标接触到开始、结束或在内部，显示源码（跳过渲染 Widget）
        if (selection.to >= r.from && selection.from <= r.to) {
            continue; 
        }

        widgets.push(Decoration.replace({
            widget: new MermaidWidget(doc.slice(r.contentFrom, r.contentTo)),
            block: true
        }).range(r.from, r.to));
    }

    return Decoration.set(widgets);
}

/**
 * 状态字段 (State Field)
 * 定义 Mermaid 扩展的状态管理逻辑
 */
export const mermaidStateField = StateField.define({
    create(state) {
        return computeMermaidDecorations(state);
    },
    update(decorations, transaction) {
        // 当文档改变或选择改变时重新计算
        // (因为需要根据光标位置切换 源码/预览 模式)
        if (transaction.docChanged || transaction.selection) {
            return computeMermaidDecorations(transaction.state);
        }
        return decorations;
    },
    provide: (f) => EditorView.decorations.from(f),
});
