/**
 * Deve-Note Web Initialization
 * 
 * Initializes optional browser-side registries.
 */
if (typeof window !== 'undefined') {
    window.deve_code_actions = window.deve_code_actions || [];
    window.deve_i18n = window.deve_i18n || {
        locale: "en-US",
        editor: {
            copyCode: "Copy Code",
            moreActions: "More Actions",
            noActionsAvailable: "No actions available",
            mermaidError: "Mermaid Error",
        },
    };
}
