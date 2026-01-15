import { WidgetType, Decoration, EditorView } from "@codemirror/view";
import { StateField } from "@codemirror/state";

/**
 * Image Widget (图片组件)
 * 
 * 在编辑器中渲染图片。
 * 当光标不在相关语法区域时显示图片，否则显示源码。
 */
class ImageWidget extends WidgetType {
  constructor(url, title) {
    super();
    this.url = url;
    this.title = title;
  }

  eq(other) {
    return this.url === other.url && this.title === other.title;
  }

  toDOM() {
    let container = document.createElement("span");
    container.className = "cm-image-widget inline-block my-2";
    
    let img = document.createElement("img");
    img.src = this.url;
    if (this.title) img.title = this.title;
    img.alt = this.title || "Image";
    
    // 基础样式：限制最大宽度，圆角，阴影
    img.className = "max-w-full h-auto rounded shadow-sm border border-gray-200";
    img.style.maxHeight = "400px"; 

    // [Fix RangeError] Handle selection manually
    container.onclick = (e) => {
        // e.preventDefault(); // Don't prevent default completely (image drag?)
        // but ensure cursor position is set to avoid "dead zones"
        const pos = window._debug_view.posAtDOM(container);
        if (pos !== null) {
            window._debug_view.dispatch({ selection: { anchor: pos } });
            window._debug_view.focus();
        }
    };

    container.appendChild(img);
    return container;
  }

  ignoreEvent() {
    return true;
  }
}

/**
 * 查找图片范围
 * 
 * 匹配标准 Markdown 图片语法: ![alt](url "title")
 * 支持简单的正则匹配，不处理嵌套括号等复杂情况。
 */
function findImageRanges(doc) {
  const ranges = [];
  // 正则说明：
  // !\[([^\]]*)\] : 匹配 ![alt] 部分，捕获 group 1 为 alt text
  // \(([^"\)]+)(?:\s+"([^"]+)")?\) : 匹配 (url "title") 部分
  //   group 2 为 url (排除引号和右括号)
  //   group 3 为可选的 title (双引号内)
  const regex = /!\[([^\]]*)\]\(([^" \)]+)(?:\s+"([^"]+)")?\)/g;
  
  let match;
  while ((match = regex.exec(doc)) !== null) {
    const from = match.index;
    const to = from + match[0].length;
    const alt = match[1];
    const url = match[2];
    const title = match[3] || alt; // 如果没有 tile，使用 alt 作为 title

    ranges.push({ from, to, url, title });
  }
  return ranges;
}

/**
 * 计算图片装饰
 * 
 * 仅当光标未触碰图片源码时才显示 Widget。
 */
function computeImageDecorations(state) {
  let widgets = [];
  const doc = state.doc.toString();
  const selection = state.selection.main;
  
  const ranges = findImageRanges(doc);

  for (const { from, to, url, title } of ranges) {
    // 简单判定：只要光标触碰到 [from, to] 区域，就显示源码
    // 包括光标刚好在前后边缘
    const isCursorTouching = selection.head >= from && selection.head <= to;
    
    if (!isCursorTouching) {
      widgets.push(
        Decoration.replace({
            widget: new ImageWidget(url, title),
            block: false // 行内替换，或者设为 true 强制换行（视需求而定）
        }).range(from, to)
      );
    }
  }
  
  return Decoration.set(widgets);
}

/**
 * Image State Field
 * 
 * 注册到 CodeMirror 的状态字段
 */
export const imageStateField = StateField.define({
  create(state) {
    return computeImageDecorations(state);
  },
  update(decorations, transaction) {
    if (transaction.docChanged || transaction.selection) {
      return computeImageDecorations(transaction.state);
    }
    return decorations;
  },
  provide: (f) => EditorView.decorations.from(f),
});
