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

    function getRegisteredInitBridgeGlobal(name) {
        const bridge = window.__deveWebBridge;
        if (!bridge || typeof bridge.get !== "function") {
            throw new Error(`web bridge registry unavailable before reading ${name}`);
        }
        return bridge.get(name);
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

    const existingCodeActions = getRegisteredInitBridgeGlobal("deve_code_actions");
    const existingI18n = getRegisteredInitBridgeGlobal("deve_i18n");
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
