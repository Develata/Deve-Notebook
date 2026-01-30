import { WidgetType, Decoration, ViewPlugin } from "@codemirror/view";
import { syntaxTree } from "@codemirror/language";
import { showMenu } from "./code_menu.js";
import { ICON_COPY, ICON_CHECK, ICON_ELLIPSIS } from "./code_icons.js";

/**
 * Code Toolbar Widget (代码块工具栏)
 * 
 * Renders a floating toolbar (Copy / Menu) on the top-right of Fenced Code blocks.
 * Plugin actions can be registered via `window.deve_code_actions`.
 */
class CodeToolbarWidget extends WidgetType {
    constructor(language, from, to) {
        super();
        this.language = language;
        this.from = from; // Content start
        this.to = to;     // Content end
    }

    eq(other) {
        return this.language === other.language && 
               this.from === other.from && 
               this.to === other.to;
    }

    toDOM(view) {
        const container = document.createElement("div");
        container.className = "cm-code-toolbar flex items-center gap-1 absolute right-2 top-2 z-20 p-0.5 rounded bg-white/80 dark:bg-gray-800/80 backdrop-blur-sm border border-gray-200 dark:border-gray-700 shadow-sm";
        container.style.userSelect = "none";

        // Copy Button
        const copyBtn = document.createElement("button");
        copyBtn.className = "p-0.5 rounded hover:bg-gray-200 dark:hover:bg-gray-700 text-gray-500 transition-colors flex items-center justify-center";
        copyBtn.title = "Copy Code";
        copyBtn.innerHTML = ICON_COPY;

        copyBtn.onclick = async (e) => {
            e.preventDefault();
            e.stopPropagation();
            try {
                await navigator.clipboard.writeText(view.state.sliceDoc(this.from, this.to));
                const original = copyBtn.innerHTML;
                copyBtn.innerHTML = ICON_CHECK;
                setTimeout(() => copyBtn.innerHTML = original, 2000);
            } catch (err) {
                console.error("Failed to copy:", err);
            }
        };

        // Menu Button
        const menuBtn = document.createElement("button");
        menuBtn.className = "p-0.5 rounded hover:bg-gray-200 dark:hover:bg-gray-700 text-gray-500 transition-colors relative group flex items-center justify-center";
        menuBtn.title = "More Actions";
        menuBtn.innerHTML = ICON_ELLIPSIS;
        menuBtn.onclick = (e) => {
            e.preventDefault();
            e.stopPropagation();
            showMenu(menuBtn, { code: view.state.sliceDoc(this.from, this.to), language: this.language, view });
        };

        container.appendChild(copyBtn);
        container.appendChild(menuBtn);
        return container;
    }

    ignoreEvent() {
        return false;
    }
}

/**
 * Code Toolbar Plugin
 */
export const codeToolbarPlugin = ViewPlugin.fromClass(
    class {
        constructor(view) {
            this.decorations = this.computeDecorations(view);
        }

        update(update) {
            if (update.docChanged || update.viewportChanged) {
                this.decorations = this.computeDecorations(update.view);
            }
        }

        computeDecorations(view) {
            let widgets = [];
            const { from, to } = view.viewport;
            const tree = syntaxTree(view.state);

            tree.iterate({
                from,
                to,
                enter: (node) => {
                    if (node.name === "FencedCode") {
                        const doc = view.state.doc;
                        const startLine = doc.lineAt(node.from);
                        const endLine = doc.lineAt(node.to);
                        
                        if (startLine.number >= endLine.number) return;

                        let language = "";
                        let infoNode = node.node.getChild("CodeInfo");
                        if (infoNode) {
                            language = view.state.sliceDoc(infoNode.from, infoNode.to);
                        }

                        const contentStart = startLine.to + 1;
                        const contentEnd = Math.max(contentStart, doc.line(endLine.number).from - 1);

                        widgets.push(Decoration.widget({
                            widget: new CodeToolbarWidget(language, contentStart, contentEnd),
                            side: 1
                        }).range(startLine.from));
                    }
                },
            });

            return Decoration.set(widgets);
        }
    },
    {
        decorations: (v) => v.decorations,
    }
);
