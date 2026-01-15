/**
 * Table Widget for CodeMirror (表格组件)
 * 
 * 作用: 当光标不在表格区域时，将 Markdown 表格渲染为 HTML <table> 元素。
 */

import { WidgetType, Decoration, EditorView } from "@codemirror/view";
import { StateField } from "@codemirror/state";
import { findTableRanges } from "./table_parser.js";

/**
 * Table Widget (表格视图组件)
 */
export class TableWidget extends WidgetType {
    constructor(tableData) {
        super();
        this.tableData = tableData;
    }
    
    toDOM(view) {
        const table = document.createElement('table');
        table.className = 'cm-table-widget w-full border-collapse my-4 text-sm';
        
        // 渲染表头
        const thead = document.createElement('thead');
        const headerRow = document.createElement('tr');
        console.log("Header Data:", this.tableData.header);
        
        this.tableData.header.forEach((cell, i) => {
            const th = document.createElement('th');
            th.textContent = cell;
            th.className = "border border-gray-300 px-4 py-2 bg-gray-100 font-semibold";
            th.style.textAlign = this.tableData.alignments[i] || 'left';
            headerRow.appendChild(th);
        });
        thead.appendChild(headerRow);
        table.appendChild(thead);
        
        // 渲染表体
        const tbody = document.createElement('tbody');
        this.tableData.body.forEach((row, rowIndex) => {
            const tr = document.createElement('tr');
            tr.className = rowIndex % 2 === 0 ? "bg-white" : "bg-gray-50";
            
            row.forEach((cell, i) => {
                const td = document.createElement('td');
                td.textContent = cell;
                td.className = "border border-gray-300 px-4 py-2";
                td.style.textAlign = this.tableData.alignments[i] || 'left';
                tr.appendChild(td);
            });
            tbody.appendChild(tr);
        });
        table.appendChild(tbody);
        
        
        // [Fix RangeError] Handle selection manually
        table.onclick = (e) => {
            e.preventDefault();
            const pos = view.posAtDOM(table);
            if (pos !== null) {
                view.dispatch({ selection: { anchor: pos } });
                view.focus();
            }
        };
        
        return table;
    }
    
    eq(other) {
        return JSON.stringify(this.tableData) === JSON.stringify(other.tableData);
    }

    ignoreEvent() {
        return true;
    }
}

/**
 * 计算表格装饰
 */
function computeTableDecorations(state) {
    const widgets = [];
    const doc = state.doc.toString();
    const selection = state.selection.main;
    
    // 使用 parser 模块查找表格
    const ranges = findTableRanges(doc);
    
    for (const range of ranges) {
        // 检查光标是否在表格范围内
        const isCursorInside = selection.head >= range.from && selection.head <= range.to;
        
        if (!isCursorInside) {
            widgets.push(
                Decoration.replace({
                    widget: new TableWidget(range.data),
                    block: true
                }).range(range.from, range.to)
            );
        }
    }
    
    return Decoration.set(widgets);
}

/**
 * Table State Field (表格状态字段)
 */
export const tableStateField = StateField.define({
    create(state) {
        return computeTableDecorations(state);
    },
    update(decorations, transaction) {
        if (transaction.docChanged || transaction.selection) {
            return computeTableDecorations(transaction.state);
        }
        return decorations;
    },
    provide: (f) => EditorView.decorations.from(f)
});
