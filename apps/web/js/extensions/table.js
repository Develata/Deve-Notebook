import { WidgetType, EditorView } from "@codemirror/view";
import { StateField } from "@codemirror/state";
import { editorCopy } from "../i18n.js";
import { renderInline } from "./inline_renderer.js";
import {
  createRenderFieldState,
  taggedCompanion,
  taggedReplace,
  updateRenderFieldState,
} from "./render_field.js";
import { reportReplaceFailure } from "./render_effects.js";

export const MAX_COMPANION_TABLE_LENGTH = 131072;
export const MAX_COMPANION_TABLE_CELLS = 2000;

function tableCellCount(tableData) {
  return tableData.header.length
    + tableData.body.reduce((total, row) => total + row.length, 0);
}

function showCompanionStatus(wrapper, key) {
  wrapper.classList.add("cm-active-preview-status");
  wrapper.textContent = editorCopy(key);
}

export class TableWidget extends WidgetType {
  constructor(range, mode) {
    super();
    this.range = range;
    this.tableData = range.data;
    this.sourceText = range.sourceText;
    this.mode = mode;
  }

  eq(other) {
    return this.range.key === other.range.key
      && this.sourceText === other.sourceText
      && this.mode === other.mode;
  }

  toDOM(view) {
    const wrapper = document.createElement("div");
    wrapper.className = "cm-render-widget-shell cm-table-render-shell";
    if (this.mode === "companion") {
      wrapper.classList.add("cm-active-preview");
      wrapper.dataset.deveActivePreview = "table";
      wrapper.setAttribute("aria-hidden", "true");
    }

    if (
      this.mode === "companion"
      && (
        this.sourceText.length > MAX_COMPANION_TABLE_LENGTH
        || tableCellCount(this.tableData) > MAX_COMPANION_TABLE_CELLS
      )
    ) {
      showCompanionStatus(wrapper, "companionPaused");
      return wrapper;
    }

    try {
      const table = document.createElement("table");
      table.className = "cm-table-widget";

      const thead = document.createElement("thead");
      const headerRow = document.createElement("tr");
      this.tableData.header.forEach((cell, i) => {
        const th = document.createElement("th");
        renderInline(cell, th);
        th.style.textAlign = this.tableData.alignments[i] || "left";
        headerRow.appendChild(th);
      });
      thead.appendChild(headerRow);
      table.appendChild(thead);

      const tbody = document.createElement("tbody");
      this.tableData.body.forEach((row) => {
        const tr = document.createElement("tr");
        row.forEach((cell, i) => {
          const td = document.createElement("td");
          renderInline(cell, td);
          td.style.textAlign = this.tableData.alignments[i] || "left";
          tr.appendChild(td);
        });
        tbody.appendChild(tr);
      });
      table.appendChild(tbody);
      wrapper.appendChild(table);
    } catch (_error) {
      if (this.mode === "companion") showCompanionStatus(wrapper, "renderError");
      else reportReplaceFailure(view, this.range);
    }

    if (this.mode === "replace") {
      wrapper.onclick = (event) => {
        event.preventDefault();
        view.dispatch({ selection: { anchor: this.range.from } });
        view.focus();
      };
    }
    return wrapper;
  }

  ignoreEvent() {
    return true;
  }
}

function buildTableDecorations({ state, range, revealed, companion }) {
  if (revealed && !companion) return [];
  const mode = companion ? "companion" : "replace";
  const widget = new TableWidget(range, mode);
  return companion
    ? [taggedCompanion(state, range, widget)]
    : [taggedReplace(range, widget, true)];
}

export const tableStateField = StateField.define({
  create(state) {
    return createRenderFieldState(state, "table", buildTableDecorations);
  },
  update(value, transaction) {
    return updateRenderFieldState(
      value,
      transaction,
      "table",
      buildTableDecorations,
      { refreshOnTheme: true },
    );
  },
  provide: (field) => EditorView.decorations.from(field, (value) => value.decorations),
});

export { tableCellCount };
