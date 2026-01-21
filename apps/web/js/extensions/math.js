import { WidgetType, Decoration, EditorView } from "@codemirror/view";
import { StateField } from "@codemirror/state";
import { findMathRanges } from "./utils.js";

// --- 辅助函数: 计算行的引用深度 ---
function getLineQuoteDepth(lineText) {
    let depth = 0;
    for (let i = 0; i < lineText.length; i++) {
        const char = lineText[i];
        if (char === '>') depth++;
        else if (char !== ' ' && char !== '\t') break;
    }
    return depth;
}

// --- Math Widget (数学公式组件) ---
export class MathWidget extends WidgetType {
  constructor(content, isBlock, quoteDepth = 0) {
    super();
    this.content = content;
    this.isBlock = isBlock;
    this.quoteDepth = quoteDepth;  // [NEW] 嵌套深度
  }
  
  toDOM(view) {
    let span = document.createElement("span");
    span.className = "cm-math-widget" + (this.isBlock ? " cm-block-math" : "");
    try {
      if (window.katex) {
        // 使用 KaTeX 渲染
        window.katex.render(this.content, span, {
          throwOnError: false,
          displayMode: this.isBlock,
        });
      } else {
        // 降级处理：如果没有加载 KaTeX，直接显示源码
        span.innerText =
          (this.isBlock ? "$$" : "$") +
          this.content +
          (this.isBlock ? "$$" : "$"); 
      }
    } catch (e) {
      span.innerText = "Error";
    }

    // [Block Math] 使用 wrapper + padding 代替 margin
    // 这样 offsetHeight 能正确包含间距，确保行号对齐
    if (this.isBlock) {
        const wrapper = document.createElement('div');
        wrapper.style.paddingTop = '1rem';
        wrapper.style.paddingBottom = '1rem';
        
        // [NEW] 应用嵌套深度样式
        if (this.quoteDepth > 0) {
            const effectiveDepth = Math.min(this.quoteDepth, 5);
            wrapper.classList.add(`cm-nested-math-depth-${effectiveDepth}`);
        }
        
        wrapper.appendChild(span);
        
        // [Fix RangeError] Handle selection manually
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

    return span;
  }

  ignoreEvent() {
      // Only ignore events for Block Math to prevent cursor crashes
      // Inline math behaves like text usually
      return this.isBlock;
  }
}

/**
 * 计算数学公式装饰
 */
function computeMathDecorations(state) {
  let widgets = [];
  let doc = state.doc.toString();
  let selection = state.selection.main;
  
  // 使用共享解析器查找范围
  const ranges = findMathRanges(doc);

  for (let r of ranges) {
      const isBlock = r.type === "BLOCK";
      const content = doc.slice(r.contentFrom, r.contentTo);
      
      const isCursorTouching = selection.head >= r.from && selection.head <= r.to;
      
      // [NEW] 计算公式所在行的引用深度
      let quoteDepth = 0;
      if (isBlock) {
          const line = state.doc.lineAt(r.from);
          quoteDepth = getLineQuoteDepth(line.text);
      }
      
      // 仅当光标未触碰时渲染 Widget
      if (!isCursorTouching) {
        widgets.push(
            Decoration.replace({ 
                widget: new MathWidget(content, isBlock, quoteDepth),
                // [NEW] block: true 让 Block Math 支持块级光标行为
                block: isBlock
            }).range(r.from, r.to)
        );
      }
  }
  
  widgets.sort((a, b) => a.from - b.from);
  return Decoration.set(widgets);
}

/**
 * Math State Field (数学公式状态字段)
 */
export const mathStateField = StateField.define({
  create(state) {
    return computeMathDecorations(state);
  },
  update(decorations, transaction) {
    if (transaction.docChanged || transaction.selection) {
      return computeMathDecorations(transaction.state);
    }
    return decorations;
  },
  provide: (f) => EditorView.decorations.from(f),
});
