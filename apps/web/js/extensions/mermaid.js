import { WidgetType, Decoration, EditorView } from "@codemirror/view";
import { StateField } from "@codemirror/state";
import { findMermaidRanges } from "./mermaid_parser.js";
import mermaid from "mermaid"; // Static import (Bundled)

// Initialize Immediately
mermaid.initialize({
    startOnLoad: false,
    theme: 'default',
    securityLevel: 'loose',
});

class MermaidWidget extends WidgetType {
    constructor(code) {
        super();
        this.code = code;
        this.id = "mermaid-" + Math.random().toString(36).substr(2, 9);
    }

    toDOM(view) {
        const wrapper = document.createElement("div");
        wrapper.className = "cm-mermaid-widget flex justify-center items-center bg-white rounded-lg border border-gray-100 my-2 shadow-sm overflow-hidden";
        
        // Calculate height based on code lines to match source block
        const lineCount = this.code.split('\n').length;
        // Use default line height (fallback to 22px) + some padding buffer if needed
        const lineHeight = view.defaultLineHeight || 22; 
        // Add a bit of padding (e.g. 1 line) to avoid being too cramped
        const height = (lineCount * lineHeight); 
        
        wrapper.style.height = height + "px";
        wrapper.style.minHeight = height + "px"; // Force consistency
        
        const container = document.createElement("div");
        // FIX: Avoid ID collision with Mermaid's internal temp element
        container.id = this.id + "-cont"; 
        container.style.width = "100%"; 
        container.style.height = "100%"; // Fill wrapper
        container.className = "flex justify-center items-center"; // Center SVG
        wrapper.appendChild(container);

        // Schedule render check
        this.scheduleRender(container);

        // [Fix RangeError] Handle selection manually to avoid CM posAtCoords crashing on complex SVG
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

    scheduleRender(container) {
        const checkDOM = () => {
             requestAnimationFrame(() => {
                 this.renderGraph(container);
             });
        };
        checkDOM();
    }

    renderGraph(container) {
        container.innerHTML = "";
        
        try {
            mermaid.render(this.id, this.code)
                .then(({ svg }) => {
                    container.innerHTML = svg;
                    // Ensure the SVG fits into the fixed height container
                    const svgEl = container.querySelector('svg');
                    if(svgEl) {
                        // Force SVG to fill the container defined by code lines
                        svgEl.style.width = "100%";
                        svgEl.style.height = "100%"; 
                        svgEl.style.maxWidth = "100%";
                        svgEl.style.display = "block"; 
                        
                        // Remove explicit dimensions to allow scaling
                        svgEl.removeAttribute('height'); 
                        svgEl.removeAttribute('width');  
                        
                        // CRITICAL: Ensure proportional scaling (no squashing)
                        // This allows the user to "zoom" by adding newlines (increasing container height)
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

    showError(container, error) {
        container.innerText = "Mermaid Error:\n" + error.message;
        container.className = "text-red-500 font-mono text-xs p-2 bg-red-50 rounded";
        const pre = document.createElement("pre");
        pre.className = "mt-2 text-gray-400 whitespace-pre-wrap";
        pre.innerText = this.code;
        container.appendChild(pre);
    }

    ignoreEvent() { return true; }
}

function computeMermaidDecorations(state) {
    let widgets = [];
    let doc = state.doc.toString();
    const ranges = findMermaidRanges(doc);
    const selection = state.selection.main; 

    for (let r of ranges) {
        // [Interaction Fix]
        // Check for ANY overlap between selection and the block range
        // If cursor is touching the start or end, or inside, show source.
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

export const mermaidStateField = StateField.define({
    create(state) {
        return computeMermaidDecorations(state);
    },
    update(decorations, transaction) {
        // Recompute on selection change or doc change
        if (transaction.docChanged || transaction.selection) {
            return computeMermaidDecorations(transaction.state);
        }
        return decorations;
    },
    provide: (f) => EditorView.decorations.from(f),
});
