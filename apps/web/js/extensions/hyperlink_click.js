/**
 * hyperlink_click.js - Ctrl+Click 链接跳转扩展
 *
 * 实现编辑器内 Ctrl+Click (Windows/Linux) 或 Cmd+Click (macOS) 打开链接功能。
 *
 * ## 设计原则
 * - 不干扰正常编辑：普通点击仅移动光标
 * - 精准识别：通过语法树定位链接节点，而非正则匹配
 * - 安全打开：使用 noopener noreferrer 防止 tabnabbing 攻击
 *
 * ## Invariants
 * - 仅在 Ctrl/Meta 键按下时响应
 * - 仅处理 Link、URL、Autolink 节点
 */

import { ViewPlugin } from "@codemirror/view";
import { syntaxTree } from "@codemirror/language";

/**
 * 从 Link 节点中提取 URL
 * Link 节点结构: [text](url) 或 [text][ref]
 * 我们需要找到其中的 URL 子节点
 */
function extractUrlFromLink(state, linkNode) {
  let url = null;

  // 遍历 Link 节点的子节点寻找 URL
  const cursor = linkNode.cursor();
  if (cursor.firstChild()) {
    do {
      if (cursor.name === "URL") {
        url = state.doc.sliceString(cursor.from, cursor.to);
        break;
      }
    } while (cursor.nextSibling());
  }

  return url;
}

/**
 * 在指定位置查找链接相关节点
 * @returns {{ type: string, url: string } | null}
 */
function findLinkAtPos(state, pos) {
  const tree = syntaxTree(state);
  let result = null;

  tree.iterate({
    from: pos,
    to: pos,
    enter: (node) => {
      // Autolink: <https://example.com>
      if (node.name === "Autolink") {
        const text = state.doc.sliceString(node.from, node.to);
        // 移除尖括号
        const url = text.replace(/^<|>$/g, "");
        result = { type: "autolink", url };
        return false; // 停止遍历
      }

      // URL 节点 (可能是独立的或在 Link 内)
      if (node.name === "URL") {
        const url = state.doc.sliceString(node.from, node.to);
        result = { type: "url", url };
        return false;
      }

      // Link 节点: [text](url)
      if (node.name === "Link") {
        const url = extractUrlFromLink(state, node.node);
        if (url) {
          result = { type: "link", url };
          return false;
        }
      }
    },
  });

  return result;
}

/**
 * 安全地打开 URL
 * 使用 noopener noreferrer 防止安全问题
 */
function safeOpenUrl(url) {
  // 补全协议 (如果缺失)
  let finalUrl = url;
  if (!/^https?:\/\//i.test(url) && !url.startsWith("mailto:")) {
    finalUrl = "https://" + url;
  }

  // 使用 noopener noreferrer 安全打开
  const newWindow = window.open(finalUrl, "_blank", "noopener,noreferrer");
  if (newWindow) {
    newWindow.opener = null; // 额外安全措施
  }
}

/**
 * Hyperlink Click Plugin
 * 监听 mousedown 事件，在 Ctrl/Meta 按下时打开链接
 */
export const hyperlinkClickPlugin = ViewPlugin.fromClass(
  class {
    constructor(view) {
      this.view = view;
      this.handleMouseDown = this.handleMouseDown.bind(this);
      view.dom.addEventListener("mousedown", this.handleMouseDown);
    }

    handleMouseDown(event) {
      // 仅处理 Ctrl (Win/Linux) 或 Meta (macOS) + 左键点击
      if (!event.ctrlKey && !event.metaKey) return;
      if (event.button !== 0) return; // 左键

      // 获取点击位置对应的文档位置
      const pos = this.view.posAtCoords({ x: event.clientX, y: event.clientY });
      if (pos === null) return;

      // 查找该位置的链接
      const linkInfo = findLinkAtPos(this.view.state, pos);
      if (!linkInfo || !linkInfo.url) return;

      // 阻止默认行为 (移动光标) 和冒泡
      event.preventDefault();
      event.stopPropagation();

      // 打开链接
      safeOpenUrl(linkInfo.url);
    }

    destroy() {
      this.view.dom.removeEventListener("mousedown", this.handleMouseDown);
    }
  }
);
