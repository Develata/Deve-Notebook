import { WidgetType, EditorView } from "@codemirror/view";
import { StateField } from "@codemirror/state";
import mermaid from "mermaid";
import { editorCopy } from "../i18n.js";
import {
  createRenderFieldState,
  taggedCompanion,
  taggedReplace,
  updateRenderFieldState,
} from "./render_field.js";
import {
  currentRenderTheme,
  reportReplaceFailure,
} from "./render_effects.js";

export const MAX_COMPANION_MERMAID_LENGTH = 32768;
export const COMPANION_DEBOUNCE_MS = 200;
const CONTROLLER = Symbol("deveMermaidController");
const coordinators = new WeakMap();
let configuredTheme = null;
let widgetSequence = 0;

export class LatestTaskCoordinator {
  constructor() {
    this.running = false;
    this.pending = null;
  }

  enqueue(task) {
    const token = { cancelled: false };
    if (this.pending) this.pending.token.cancelled = true;
    this.pending = { task, token };
    void this.drain();
    return token;
  }

  cancel(token) {
    if (!token) return;
    token.cancelled = true;
    if (this.pending?.token === token) this.pending = null;
  }

  async drain() {
    if (this.running) return;
    this.running = true;
    try {
      while (this.pending) {
        const item = this.pending;
        this.pending = null;
        if (!item.token.cancelled) await item.task(item.token);
      }
    } finally {
      this.running = false;
      if (this.pending) void this.drain();
    }
  }
}

function coordinatorFor(view) {
  let coordinator = coordinators.get(view);
  if (!coordinator) {
    coordinator = new LatestTaskCoordinator();
    coordinators.set(view, coordinator);
  }
  return coordinator;
}

function mermaidTheme(themeKey) {
  return themeKey === "night" ? "dark" : "default";
}

function ensureMermaidTheme(themeKey) {
  const theme = mermaidTheme(themeKey);
  if (configuredTheme === theme) return;
  mermaid.initialize({
    startOnLoad: false,
    theme,
    securityLevel: "strict",
  });
  configuredTheme = theme;
}

function sourceLineHeight(view) {
  const computed = window.getComputedStyle(view.contentDOM).lineHeight;
  const height = Number.parseFloat(computed);
  return Number.isFinite(height) && height > 0 ? height : (view.defaultLineHeight || 22);
}

function showStatus(wrapper, key) {
  wrapper.classList.add("cm-active-preview-status");
  wrapper.textContent = editorCopy(key);
}

export class MermaidWidget extends WidgetType {
  constructor(range, mode, themeKey) {
    super();
    this.range = range;
    this.code = range.content;
    this.mode = mode;
    this.themeKey = themeKey;
    this.id = `deve-mermaid-${++widgetSequence}`;
  }

  eq(other) {
    return this.range.key === other.range.key
      && this.code === other.code
      && this.mode === other.mode
      && this.themeKey === other.themeKey;
  }

  toDOM(view) {
    const wrapper = document.createElement("div");
    wrapper.className = "cm-render-widget-shell cm-mermaid-widget";
    if (this.mode === "replace") wrapper.dataset.noEdgeSwipe = "true";
    if (this.mode === "companion") {
      wrapper.classList.add("cm-active-preview");
      wrapper.dataset.deveActivePreview = "mermaid";
      wrapper.setAttribute("aria-hidden", "true");
    } else {
      const sourceHeight = this.code.split("\n").length * sourceLineHeight(view);
      wrapper.style.minHeight = `${sourceHeight}px`;
    }

    if (this.mode === "companion" && this.code.length > MAX_COMPANION_MERMAID_LENGTH) {
      showStatus(wrapper, "companionPaused");
      return wrapper;
    }

    const container = document.createElement("div");
    container.className = "cm-mermaid-content";
    wrapper.appendChild(container);

    const controller = {
      cancelled: false,
      timer: null,
      frame: null,
      token: null,
      generation: 1,
      coordinator: this.mode === "companion" ? coordinatorFor(view) : null,
    };
    wrapper[CONTROLLER] = controller;

    const schedule = () => {
      if (controller.cancelled) return;
      const renderTask = async (token) => {
        const generation = controller.generation;
        if (token.cancelled || controller.cancelled || !wrapper.isConnected) return;
        try {
          ensureMermaidTheme(this.themeKey);
          const { svg } = await mermaid.render(this.id, this.code);
          if (
            token.cancelled
            || controller.cancelled
            || controller.generation !== generation
            || !wrapper.isConnected
          ) return;

          container.innerHTML = svg;
          const svgElement = container.querySelector("svg");
          if (svgElement) {
            svgElement.classList.add("mermaid");
            svgElement.removeAttribute("height");
            svgElement.removeAttribute("width");
            svgElement.setAttribute("preserveAspectRatio", "xMidYMid meet");
          }
          view.requestMeasure();
        } catch (_error) {
          if (token.cancelled || controller.cancelled || !wrapper.isConnected) return;
          if (this.mode === "companion") showStatus(wrapper, "renderError");
          else reportReplaceFailure(view, this.range);
          view.requestMeasure();
        }
      };
      if (controller.coordinator) {
        controller.token = controller.coordinator.enqueue(renderTask);
      } else {
        controller.token = { cancelled: false };
        void renderTask(controller.token);
      }
    };

    if (this.mode === "companion") {
      controller.timer = setTimeout(schedule, COMPANION_DEBOUNCE_MS);
    } else {
      controller.frame = requestAnimationFrame(schedule);
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

  destroy(dom) {
    const controller = dom?.[CONTROLLER];
    if (!controller) return;
    controller.cancelled = true;
    controller.generation++;
    if (controller.timer !== null) clearTimeout(controller.timer);
    if (controller.frame !== null) cancelAnimationFrame(controller.frame);
    if (controller.coordinator) controller.coordinator.cancel(controller.token);
    else if (controller.token) controller.token.cancelled = true;
    delete dom[CONTROLLER];
  }

  ignoreEvent() {
    return true;
  }
}

function buildMermaidDecorations({ state, range, revealed, companion }) {
  if (revealed && !companion) return [];
  const mode = companion ? "companion" : "replace";
  const widget = new MermaidWidget(range, mode, currentRenderTheme());
  return companion
    ? [taggedCompanion(state, range, widget)]
    : [taggedReplace(range, widget, true)];
}

export const mermaidStateField = StateField.define({
  create(state) {
    return createRenderFieldState(state, "mermaid", buildMermaidDecorations);
  },
  update(value, transaction) {
    return updateRenderFieldState(
      value,
      transaction,
      "mermaid",
      buildMermaidDecorations,
      { refreshOnTheme: true },
    );
  },
  provide: (field) => EditorView.decorations.from(field, (value) => value.decorations),
});

ensureMermaidTheme(currentRenderTheme());
