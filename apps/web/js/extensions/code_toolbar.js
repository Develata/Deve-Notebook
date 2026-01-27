import { WidgetType, Decoration, ViewPlugin } from "@codemirror/view";
import { syntaxTree } from "@codemirror/language";

/**
 * Code Toolbar Widget (代码块工具栏)
 * 
 * Renders a floating toolbar (Copy / Menu) on the top-right of Fenced Code blocks.
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
        // [FIX] Further reduced size:
        // gap-2 -> gap-1
        // px-1 py-0.5 -> p-0.5 (smaller padding)
        container.className = "cm-code-toolbar flex items-center gap-1 absolute right-2 top-2 z-20 p-0.5 rounded bg-white/80 dark:bg-gray-800/80 backdrop-blur-sm border border-gray-200 dark:border-gray-700 shadow-sm";
        container.style.userSelect = "none";

        // --- Copy Button ---
        const copyBtn = document.createElement("button");
        // p-1 -> p-0.5
        copyBtn.className = "p-0.5 rounded hover:bg-gray-200 dark:hover:bg-gray-700 text-gray-500 transition-colors flex items-center justify-center";
        copyBtn.title = "Copy Code";
        // SVG 14 -> 12
        copyBtn.innerHTML = `
            <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
                <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
            </svg>
        `;

        copyBtn.onclick = async (e) => {
            e.preventDefault();
            e.stopPropagation(); // Prevent editor focus change

            const content = view.state.sliceDoc(this.from, this.to);
            
            try {
                await navigator.clipboard.writeText(content);
                
                // Feedback animation
                const originalIcon = copyBtn.innerHTML;
                copyBtn.innerHTML = `
                    <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="green" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <polyline points="20 6 9 17 4 12"></polyline>
                    </svg>
                `;
                setTimeout(() => {
                    copyBtn.innerHTML = originalIcon;
                }, 2000);
            } catch (err) {
                console.error("Failed to copy:", err);
            }
        };

        // --- Menu Button (Ellipsis) ---
        const menuBtn = document.createElement("button");
        // p-1 -> p-0.5
        menuBtn.className = "p-0.5 rounded hover:bg-gray-200 dark:hover:bg-gray-700 text-gray-500 transition-colors relative group flex items-center justify-center";
        menuBtn.title = "More Actions";
        // SVG 14 -> 12
        menuBtn.innerHTML = `
            <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <circle cx="12" cy="12" r="1"></circle>
                <circle cx="19" cy="12" r="1"></circle>
                <circle cx="5" cy="12" r="1"></circle>
            </svg>
        `;


        menuBtn.onclick = (e) => {
            e.preventDefault();
            e.stopPropagation();
            
            // [FIX] Always show tooltip for now to prove click works
            // In future, this will check window.deve_code_actions
            const tooltip = document.createElement("div");
            tooltip.className = "absolute top-full right-0 mt-1 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 shadow-lg rounded px-2 py-1 text-xs text-gray-500 whitespace-nowrap z-50";
            tooltip.innerText = "No actions available";
            menuBtn.appendChild(tooltip);
            
            // Auto remove tooltip
            setTimeout(() => tooltip.remove(), 2000);
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
