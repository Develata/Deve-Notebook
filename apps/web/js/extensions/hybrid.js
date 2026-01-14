import { ViewPlugin, Decoration } from "@codemirror/view";
import { syntaxTree } from "@codemirror/language";
import { findMathRanges, findFrontmatterRange } from "./utils.js";

/**
 * Hybrid Plugin (混合插件)
 * 
 * 作用:
 * 1. 隐藏 Markdown 的部分语法标记 (如 Header 的 #, Emphasis 的 *)
 * 2. 渲染 Checkbox
 * 3. 避免在数学公式内部进行处理
 */
export const hybridPlugin = ViewPlugin.fromClass(
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
      let widgets = [];
      const { from, to } = view.viewport;
      const selection = view.state.selection.main;
      const doc = view.state.doc.toString();
      
      // 辅助函数: 检查光标是否在范围内
      const isCursorIn = (nodeFrom, nodeTo) =>
        selection.head >= nodeFrom && selection.head <= nodeTo;

      // 1. 获取所有数学公式范围，避免处理公式内的内容
      const mathRanges = findMathRanges(doc);
      
      const isInsideMath = (nodeFrom, nodeTo) => {
        for (let r of mathRanges) {
          // 只要有重叠就视为在公式内 (简单的碰撞检测)
          if (Math.max(nodeFrom, r.from) <= Math.min(nodeTo, r.to)) return true;
        }
        return false;
      };

      // 2. Frontmatter Detection (Strict)
      // 计算一次，如果存在有效的 Frontmatter，则添加装饰
      const fm = findFrontmatterRange(doc);
      if (fm) {
          // 仅当 Frontmatter 在视口范围内时渲染
          if (fm.from <= to && fm.to >= from) {
              
              // 1. Style Background (Line Decoration for Full Width)
              // Iterate all lines from fm.from to fm.to
              for (let pos = fm.from; pos < fm.to; ) {
                  const line = view.state.doc.lineAt(pos);
                  
                  // Apply Line Decoration
                  widgets.push(
                      Decoration.line({ 
                          attributes: { class: "cm-frontmatter-block" } 
                      }).range(line.from)
                  );
                  
                  pos = line.to + 1;
              }

              // 2. Manage Delimiter Visibility
              // Only hide if cursor is NOT in the Frontmatter block
              const isCursorInFm = selection.head >= fm.from && selection.head <= fm.to;
              
              if (!isCursorInFm) {
                  // Inactive: Hide delimiters
                  widgets.push(Decoration.mark({ class: "cm-syntax-hidden" }).range(fm.from, fm.from + 3)); 
                  widgets.push(Decoration.mark({ class: "cm-syntax-hidden" }).range(fm.contentTo, fm.contentTo + 3));
              } else {
                  // Active: Distinctly style them (ensure visibility)
                  widgets.push(Decoration.mark({ class: "cm-frontmatter-delim" }).range(fm.from, fm.from + 3));
                  widgets.push(Decoration.mark({ class: "cm-frontmatter-delim" }).range(fm.contentTo, fm.contentTo + 3));
              }
          }
      }

      try {
        let tree = syntaxTree(view.state);

        tree.iterate({
          from,
          to,
          enter: (node) => {
            // DEBUG: Log node name
            // console.log("Node:", node.name, node.from, node.to);

            // 跳过数学公式区域
            if (isInsideMath(node.from, node.to)) return;
            
            // 跳过 Frontmatter 内部 (如果已检测到)
            // 避免 Frontmatter 内部的 key: value 被识别为 Setext Heading 的一部分并被隐藏/错误处理
            if (fm && node.from >= fm.from && node.to <= fm.to) return;

            // ---------------------------------------------------------
            // 2. Syntax Hiding (Hiding Marks when not active)
            // ---------------------------------------------------------
            // 隐藏标题的 # 符号 和 强调符号 * _ 和 引用符号 > 和 行内代码标记 ` 和 删除线标记 ~~
            if (node.name === "HeaderMark" || 
                node.name === "EmphasisMark" || 
                node.name === "QuoteMark" || 
                node.name === "CodeMark" || 
                node.name === "StrikethroughMark") {  // [NEW] Added StrikethroughMark
                
              const parent = node.node.parent;
              
              // 特殊处理: 如果是 CodeMark (即 `), 且父节点是 FencedCode (代码块), 则不隐藏
              if (node.name === "CodeMark" && parent && parent.name === "FencedCode") {
                  return;
              }

              // 只有当光标不在该行/区域时才隐藏
              if (parent && !isCursorIn(parent.from, parent.to)) {
                widgets.push(Decoration.mark({ class: "cm-syntax-hidden" }).range(node.from, node.to));
              }
            }
            
            // ---------------------------------------------------------
            // 3. Manual Styling Takeover (Ensure consistent visual)
            // ---------------------------------------------------------
            
            // Explicit Styling for Bold/Italic/Strikethrough
            // logic: Only apply style if cursor is NOT inside (Source Mode vs Preview Mode toggle)
            if (node.name === "StrongEmphasis") {
                 if (!isCursorIn(node.from, node.to)) {
                    widgets.push(Decoration.mark({ class: "cm-strong" }).range(node.from, node.to));
                 }
            }
            if (node.name === "Emphasis") {
                 if (!isCursorIn(node.from, node.to)) {
                    widgets.push(Decoration.mark({ class: "cm-em" }).range(node.from, node.to));
                 }
            }
            if (node.name === "Strikethrough") {
                 if (!isCursorIn(node.from, node.to)) {
                    widgets.push(Decoration.mark({ class: "cm-strikethrough" }).range(node.from, node.to));
                 }
            }
            
            // Explicit Styling for Headings
            if (node.name === "ATXHeading1") widgets.push(Decoration.mark({ class: "cm-h1" }).range(node.from, node.to));
            if (node.name === "ATXHeading2") widgets.push(Decoration.mark({ class: "cm-h2" }).range(node.from, node.to));
            if (node.name === "ATXHeading3") widgets.push(Decoration.mark({ class: "cm-h3" }).range(node.from, node.to));
            if (node.name === "ATXHeading4") widgets.push(Decoration.mark({ class: "cm-h4" }).range(node.from, node.to));
            if (node.name === "ATXHeading5") widgets.push(Decoration.mark({ class: "cm-h5" }).range(node.from, node.to));
            if (node.name === "ATXHeading6") widgets.push(Decoration.mark({ class: "cm-h6" }).range(node.from, node.to));

            // Explicit Styling for Links
            if (node.name === "Link") {
                widgets.push(Decoration.mark({ class: "cm-link" }).range(node.from, node.to));
            }
            if (node.name === "URL") {
                 widgets.push(Decoration.mark({ class: "cm-url" }).range(node.from, node.to));
            }
            
            // Explicit Styling for Blockquotes
            if (node.name === "Blockquote") {
                widgets.push(Decoration.mark({ class: "cm-blockquote" }).range(node.from, node.to));
            }
            
            // Explicit Styling for Horizontal Rules
            if (node.name === "HorizontalRule") {
                 widgets.push(Decoration.mark({ class: "cm-hr" }).range(node.from, node.to));
            }
            
            // Inline Code 样式标记
            if (node.name === "InlineCode") {
                // 添加背景色样式
                widgets.push(
                    Decoration.mark({ class: "cm-inline-code" }).range(
                        node.from,
                        node.to
                    )
                );
            }
          },
        });
      } catch (e) {
        console.warn("HybridPlugin Error:", e);
      }

      return Decoration.set(widgets.sort((a, b) => a.from - b.from));
    }
  },
  { decorations: (v) => v.decorations }
);
