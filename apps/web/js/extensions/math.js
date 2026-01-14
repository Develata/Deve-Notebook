import { WidgetType, Decoration, EditorView } from "@codemirror/view";
import { StateField } from "@codemirror/state";
import { findMathRanges } from "./utils.js";

// --- Math Widget (数学公式组件) ---
export class MathWidget extends WidgetType {
  constructor(content, isBlock) {
    super();
    this.content = content;
    this.isBlock = isBlock;
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
    return span;
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
      
      // 仅当光标未触碰时渲染 Widget
      if (!isCursorTouching) {
        widgets.push(
            Decoration.replace({ 
                widget: new MathWidget(content, isBlock) 
            }).range(r.from, r.to)
        );
      }
  }
  
  // 处理转义美元符号的逻辑 (简化版)
  // 如果需要严格隐藏 \$ 的反斜杠，可以参考之前的逻辑
  // 目前这里的重点是渲染公式，转义符暂时保留原样显示
  
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
