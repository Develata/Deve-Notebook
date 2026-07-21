import { StateEffect } from "@codemirror/state";
import { ViewPlugin } from "@codemirror/view";

export const renderFailureEffect = StateEffect.define();
export const renderThemeEffect = StateEffect.define();

export function currentRenderTheme() {
    if (typeof document === "undefined") return "warm";
    return document.documentElement?.getAttribute("data-deve-theme-pref") || "warm";
}

export function reportReplaceFailure(view, range) {
    const payload = {
        kind: range.kind,
        from: range.from,
        to: range.to,
        sourceText: range.sourceText,
    };
    queueMicrotask(() => {
        if (!view?.dom?.isConnected) return;
        view.dispatch({ effects: renderFailureEffect.of(payload) });
    });
}

export const renderThemeObserver = ViewPlugin.fromClass(class {
    constructor(view) {
        this.view = view;
        this.theme = currentRenderTheme();
        this.observer = null;
        if (typeof MutationObserver === "undefined" || typeof document === "undefined") return;

        this.observer = new MutationObserver(() => {
            const nextTheme = currentRenderTheme();
            if (nextTheme === this.theme) return;
            this.theme = nextTheme;
            this.view.dispatch({ effects: renderThemeEffect.of(nextTheme) });
        });
        this.observer.observe(document.documentElement, {
            attributes: true,
            attributeFilter: ["data-deve-theme-pref"],
        });
    }

    destroy() {
        this.observer?.disconnect();
        this.observer = null;
    }
});
