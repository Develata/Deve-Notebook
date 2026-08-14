import { WidgetType, EditorView } from "@codemirror/view";
import { StateField } from "@codemirror/state";
import { renderKatex } from "../rendering_bridge.js";
import { editorCopy } from "../i18n.js";
import { renderRangeIndexField } from "./render_range_index.js";
import {
  createRenderFieldState,
  taggedCompanion,
  taggedReplace,
  updateRenderFieldState,
} from "./render_field.js";
import { reportReplaceFailure } from "./render_effects.js";

const MAX_NESTED_MATH_DEPTH = 10;
export const MAX_COMPANION_MATH_LENGTH = 8192;

function getLineQuoteDepth(lineText) {
  let depth = 0;
  for (const char of lineText) {
    if (char === ">") depth++;
    else if (char !== " " && char !== "\t") break;
  }
  return depth;
}

function mathSourceText(content, isBlock) {
  const marker = isBlock ? "$$" : "$";
  return marker + content + marker;
}

function showCompanionStatus(wrapper, key) {
  wrapper.classList.add("cm-active-preview-status");
  wrapper.textContent = editorCopy(key);
}

export class MathWidget extends WidgetType {
  constructor(range, mode, quoteDepth = 0) {
    super();
    this.range = range;
    this.content = range.content;
    this.isBlock = range.type === "BLOCK";
    this.mode = mode;
    this.quoteDepth = quoteDepth;
  }

  eq(other) {
    return this.range.key === other.range.key
      && this.content === other.content
      && this.isBlock === other.isBlock
      && this.mode === other.mode
      && this.quoteDepth === other.quoteDepth;
  }

  toDOM(view) {
    const span = document.createElement("span");
    span.className = "cm-math-widget" + (this.isBlock ? " cm-block-math" : "");

    if (!this.isBlock) {
      const rendered = renderKatex(this.content, span, {
        throwOnError: false,
        displayMode: false,
      }, mathSourceText(this.content, false));
      if (!rendered) reportReplaceFailure(view, this.range);
      return span;
    }

    const wrapper = document.createElement("div");
    wrapper.className = "cm-render-widget-shell cm-math-render-shell";
    if (this.mode === "replace") wrapper.dataset.noEdgeSwipe = "true";
    if (this.mode === "companion") {
      wrapper.classList.add("cm-active-preview");
      wrapper.dataset.deveActivePreview = "math";
      wrapper.setAttribute("aria-hidden", "true");
    }

    if (this.quoteDepth > 0) {
      const effectiveDepth = Math.min(this.quoteDepth, MAX_NESTED_MATH_DEPTH);
      wrapper.classList.add(`cm-nested-math-depth-${effectiveDepth}`);
    }

    if (this.mode === "companion" && this.content.length > MAX_COMPANION_MATH_LENGTH) {
      showCompanionStatus(wrapper, "companionPaused");
      return wrapper;
    }

    wrapper.appendChild(span);
    const rendered = renderKatex(this.content, span, {
      throwOnError: false,
      displayMode: true,
    }, "");

    if (!rendered) {
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
    return this.isBlock;
  }
}

function buildMathDecorations({ state, range, revealed, companion }) {
  if (revealed && !companion) return [];
  const quoteDepth = range.type === "BLOCK"
    ? getLineQuoteDepth(state.doc.lineAt(range.from).text)
    : 0;
  const mode = companion ? "companion" : "replace";
  const widget = new MathWidget(range, mode, quoteDepth);
  return companion
    ? [taggedCompanion(state, range, widget)]
    : [taggedReplace(range, widget, range.type === "BLOCK")];
}

export const mathStateField = StateField.define({
  create(state) {
    return createRenderFieldState(state, "math", buildMathDecorations);
  },
  update(value, transaction) {
    return updateRenderFieldState(
      value,
      transaction,
      "math",
      buildMathDecorations,
      { refreshOnTheme: true },
    );
  },
  provide: (field) => EditorView.decorations.from(field, (value) => value.decorations),
});

export { renderRangeIndexField };
