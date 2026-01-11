/**
 * Table Widget for CodeMirror
 * 
 * Renders Markdown tables as HTML <table> elements when cursor is not inside.
 */

import { WidgetType, Decoration, EditorView } from "@codemirror/view";
import { StateField } from "@codemirror/state";

/**
 * Parse a markdown table string into structured data
 */
function parseTable(tableText) {
    const lines = tableText.trim().split('\n');
    if (lines.length < 2) return null;
    
    const parseRow = (line) => {
        // Remove leading/trailing pipes and split
        return line
            .replace(/^\|/, '')
            .replace(/\|$/, '')
            .split('|')
            .map(cell => cell.trim());
    };
    
    const headerRow = parseRow(lines[0]);
    const separatorLine = lines[1];
    
    // Validate separator row - must contain | and - characters
    // Examples: |---|---|, |-|-|, | --- | --- |, |:---:|:---:|
    if (!separatorLine.includes('|') || !separatorLine.includes('-')) return null;
    
    // Each cell in separator should only contain -, :, and spaces
    const separatorCells = parseRow(separatorLine);
    const validSeparator = separatorCells.every(cell => /^:?-+:?$/.test(cell.trim()) || cell.trim() === '');
    if (!validSeparator) return null;
    
    // Parse alignment from separator
    const alignments = separatorCells.map(sep => {
        sep = sep.trim();
        const left = sep.startsWith(':');
        const right = sep.endsWith(':');
        if (left && right) return 'center';
        if (right) return 'right';
        return 'left';
    });
    
    // Parse body rows
    const bodyRows = lines.slice(2).map(parseRow);
    
    return { header: headerRow, alignments, body: bodyRows };
}

/**
 * Widget that renders a table
 */
export class TableWidget extends WidgetType {
    constructor(tableData) {
        super();
        this.tableData = tableData;
    }
    
    toDOM() {
        const table = document.createElement('table');
        table.className = 'cm-table-widget';
        
        // Header
        const thead = document.createElement('thead');
        const headerRow = document.createElement('tr');
        this.tableData.header.forEach((cell, i) => {
            const th = document.createElement('th');
            th.textContent = cell;
            th.style.textAlign = this.tableData.alignments[i] || 'left';
            headerRow.appendChild(th);
        });
        thead.appendChild(headerRow);
        table.appendChild(thead);
        
        // Body
        const tbody = document.createElement('tbody');
        this.tableData.body.forEach(row => {
            const tr = document.createElement('tr');
            row.forEach((cell, i) => {
                const td = document.createElement('td');
                td.textContent = cell;
                td.style.textAlign = this.tableData.alignments[i] || 'left';
                tr.appendChild(td);
            });
            tbody.appendChild(tr);
        });
        table.appendChild(tbody);
        
        return table;
    }
    
    eq(other) {
        return JSON.stringify(this.tableData) === JSON.stringify(other.tableData);
    }
}

/**
 * Find table ranges in the document
 */
export function findTableRanges(doc) {
    const ranges = [];
    const lines = doc.split('\n');
    let i = 0;
    let pos = 0;
    
    while (i < lines.length) {
        const line = lines[i];
        
        // Check if this line could be a table header
        if (line.includes('|') && i + 1 < lines.length) {
            const nextLine = lines[i + 1];
            
            // Check if next line is separator (contains | and -)
            if (nextLine.includes('|') && nextLine.includes('-')) {
                const startPos = pos;
                let tableEnd = i + 1;
                
                // Find end of table
                for (let j = i + 2; j < lines.length; j++) {
                    if (lines[j].includes('|')) {
                        tableEnd = j;
                    } else {
                        break;
                    }
                }
                
                // Calculate end position
                let endPos = startPos;
                for (let j = i; j <= tableEnd; j++) {
                    endPos += lines[j].length + 1; // +1 for newline
                }
                endPos--; // Remove last newline offset
                
                const tableText = lines.slice(i, tableEnd + 1).join('\n');
                const tableData = parseTable(tableText);
                
                if (tableData) {
                    ranges.push({
                        from: startPos,
                        to: endPos,
                        data: tableData
                    });
                }
                
                // Skip processed lines
                i = tableEnd + 1;
                pos = endPos + 1;
                continue;
            }
        }
        
        pos += line.length + 1;
        i++;
    }
    
    return ranges;
}

/**
 * StateField that manages table decorations
 */
function computeTableDecorations(state) {
    const widgets = [];
    const doc = state.doc.toString();
    const selection = state.selection.main;
    
    const ranges = findTableRanges(doc);
    console.log("[TABLE] Found ranges:", ranges.length, ranges);
    
    for (const range of ranges) {
        const isCursorInside = selection.head >= range.from && selection.head <= range.to;
        console.log("[TABLE] Range:", range.from, "-", range.to, "Cursor:", selection.head, "Inside:", isCursorInside);
        
        if (!isCursorInside) {
            widgets.push(
                Decoration.replace({
                    widget: new TableWidget(range.data),
                    block: true
                }).range(range.from, range.to)
            );
        }
    }
    
    console.log("[TABLE] Total widgets:", widgets.length);
    return Decoration.set(widgets);
}

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
