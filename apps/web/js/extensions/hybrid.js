import { ViewPlugin, Decoration } from "@codemirror/view";
import { syntaxTree } from "@codemirror/language";
import { findMathRanges } from "./utils.js";
import { CheckboxWidget } from "./checkbox.js";

// --- Hybrid Plugin ---
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
      const isCursorIn = (nodeFrom, nodeTo) =>
        selection.head >= nodeFrom && selection.head <= nodeTo;

      // Use shared math parser
      const mathRanges = findMathRanges(doc);
      
      const isInsideMath = (nodeFrom, nodeTo) => {
        for (let r of mathRanges) {
          if (nodeFrom >= r.from && nodeTo <= r.to) return true;
        }
        return false;
      };

      try {
        let tree = syntaxTree(view.state);

        tree.iterate({
          from,
          to,
          enter: (node) => {
            if (isInsideMath(node.from, node.to)) return;

            // Hide Header/Emphasis Marks
            if (node.name === "HeaderMark" || node.name === "EmphasisMark") {
              const parent = node.node.parent;
              if (parent && !isCursorIn(parent.from, parent.to)) {
                widgets.push(Decoration.replace({}).range(node.from, node.to));
              }
            }

            // Checkboxes
            if (node.name === "TaskMarker") {
              const slice = view.state.sliceDoc(node.from, node.to);
              const isChecked = slice.toLowerCase().includes("x");

              if (!isCursorIn(node.from, node.to)) {
                widgets.push(
                  Decoration.replace({
                    widget: new CheckboxWidget(isChecked, node.from),
                  }).range(node.from, node.to)
                );
              }
            }

            // Frontmatter
            if (node.name === "Frontmatter") {
              widgets.push(
                Decoration.mark({ class: "cm-frontmatter" }).range(
                  node.from,
                  node.to
                )
              );
            }
          },
        });
      } catch (e) {
        console.warn(e);
      }

      return Decoration.set(widgets);
    }
  },
  { decorations: (v) => v.decorations }
);
