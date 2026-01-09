import { WidgetType, Decoration } from "@codemirror/view";

// --- Checkbox Widget ---
export class CheckboxWidget extends WidgetType {
  constructor(checked, pos) {
    super();
    this.checked = checked;
    this.pos = pos;
  }
  toDOM(view) {
    let wrap = document.createElement("span");
    wrap.className = "cm-checkbox-widget";
    let input = document.createElement("input");
    input.type = "checkbox";
    input.checked = this.checked;
    input.className = "cursor-pointer";

    input.onclick = (e) => {
      e.preventDefault();
      const valid = this.checked ? " " : "x";
      view.dispatch({
        changes: { from: this.pos + 1, to: this.pos + 2, insert: valid },
      });
    };
    wrap.appendChild(input);
    return wrap;
  }

  ignoreEvent() {
    return false;
  }
}
