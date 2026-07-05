/**
 * Deve-Note Web Initialization
 *
 * Initializes optional browser-side registries.
 */
if (typeof window !== 'undefined') {
    function registerInitBridgeGlobal(name, value, meta = {}) {
        const bridge = window.__deveWebBridge;
        if (!bridge || typeof bridge.register !== "function") {
            throw new Error(`web bridge registry unavailable before registering ${name}`);
        }
        return bridge.register(name, value, {
            runtime: "widget_bridge_runtime",
            source: "init-bootstrap",
            authority: "none",
            ...meta,
        });
    }

    const defaultI18n = {
        locale: "en-US",
        editor: {
            copyCode: "Copy Code",
            moreActions: "More Actions",
            noActionsAvailable: "No actions available",
            mermaidError: "Mermaid Error",
        },
    };

    const existingCodeActions = window.deve_code_actions;
    const existingI18n = window.deve_i18n;
    const codeActions = Array.isArray(existingCodeActions)
        ? existingCodeActions
        : [];
    const i18n = existingI18n && typeof existingI18n === "object"
        ? existingI18n
        : defaultI18n;

    registerInitBridgeGlobal("deve_code_actions", codeActions, {
        role: "code-toolbar-action-registry",
    });
    registerInitBridgeGlobal("deve_i18n", i18n, {
        role: "browser-i18n-copy-registry",
    });
}
