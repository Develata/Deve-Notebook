import { WidgetType, Decoration, EditorView } from "@codemirror/view";
import { StateField } from "@codemirror/state";
import { syntaxTree } from "@codemirror/language";

console.log("Loading Checkbox Extension (SyntaxTree Version)...");

/**
 * Checkbox Widget (复选框组件)
 * 
 * 渲染漂亮的复选框，即时响应点击事件。
 */
export class CheckboxWidget extends WidgetType {
  constructor(checked, pos) {
    super();
    this.checked = checked;
    this.pos = pos;
  }

  eq(other) {
    return this.checked === other.checked && this.pos === other.pos;
  }

  toDOM(view) {
    let wrap = document.createElement("span");
    // 基础样式
    wrap.className = "cm-checkbox-widget inline-flex items-center justify-center align-middle mr-2 select-none";
    wrap.style.width = "1.2em";
    wrap.style.height = "1.2em";
    wrap.style.verticalAlign = "middle";
    
    let input = document.createElement("input");
    input.type = "checkbox";
    input.checked = this.checked;
    // 使用 Tailwind 类 + 内联兜底
    input.className = "cursor-pointer appearance-none w-4 h-4 border border-gray-400 rounded bg-white checked:bg-blue-600 checked:border-blue-600 focus:ring-2 focus:ring-blue-200 focus:outline-none transition-all duration-200 relative";
    input.style.width = "1em";
    input.style.height = "1em";
    input.style.appearance = "none";
    input.style.border = "1px solid #ccc";
    input.style.borderRadius = "3px";
    input.style.backgroundColor = "white";
    input.style.cursor = "pointer";

    if (this.checked) {
        input.style.backgroundColor = "#2563eb"; // blue-600
        input.style.borderColor = "#2563eb";
        // SVG Data URI for Checkmark
        input.style.backgroundImage = `url("data:image/svg+xml,%3csvg viewBox='0 0 16 16' fill='white' xmlns='http://www.w3.org/2000/svg'%3e%3cpath d='M12.207 4.793a1 1 0 010 1.414l-5 5a1 1 0 01-1.414 0l-2-2a1 1 0 011.414-1.414L6.5 9.086l4.293-4.293a1 1 0 011.414 0z'/%3e%3c/svg%3e")`;
        input.style.backgroundPosition = "center";
        input.style.backgroundRepeat = "no-repeat";
        input.style.backgroundSize = "contain";
    }

    input.onclick = (e) => {
      e.preventDefault();
      const newStatusMark = this.checked ? " " : "x";
      // pos 是 '[' 的位置. 复选框内容可能是 "[ ]" 或 "[x]"
      // 我们替换 pos+1 (即 '[') 后面的字符
      view.dispatch({
        changes: { from: this.pos + 1, to: this.pos + 2, insert: newStatusMark },
      });
    };

    wrap.appendChild(input);
    return wrap;
  }

  ignoreEvent() {
    return false;
  }
}

/**
 * 计算 Checkbox 装饰 (使用 syntaxTree)
 */
function computeCheckboxDecorations(state) {
  let widgets = [];
  const selection = state.selection.main;
  
  // 使用语法树遍历，比正则更可靠 (避免代码块内的误判)
  const tree = syntaxTree(state);
  
  tree.iterate({
      enter: (node) => {
          if (node.name === "TaskMarker") {
              const from = node.from;
              const to = node.to;
              
              // 获取文本内容，例如 "[ ]" 或 "[x]"
              const slice = state.sliceDoc(from, to);
              const isChecked = slice.toLowerCase().includes("x");
              
              // 只有当光标 *不* 在 `[ ]` 或 `[x]` 内部或边缘时，才显示 Widget
              const isCursorTouching = selection.head >= from && selection.head <= to;
    
              if (!isCursorTouching) {
                  widgets.push(
                    Decoration.replace({
                        widget: new CheckboxWidget(isChecked, from),
                        inclusive: true
                    }).range(from, to)
                  );
              }
          }
      }
  });
  
  return Decoration.set(widgets);
}

/**
 * Checkbox State Field
 */
export const checkboxStateField = StateField.define({
  create(state) {
    return computeCheckboxDecorations(state);
  },
  update(decorations, transaction) {
    if (transaction.docChanged || transaction.selection) {
      return computeCheckboxDecorations(transaction.state);
    }
    return decorations;
  },
  provide: (f) => EditorView.decorations.from(f),
});
