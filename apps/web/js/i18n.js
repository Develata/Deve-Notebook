const DEFAULT_LOCALE = "en-US";

const FALLBACK_COPY = {
    "en-US": {
        editor: {
            copyCode: "Copy Code",
            moreActions: "More Actions",
            noActionsAvailable: "No actions available",
            mermaidError: "Mermaid Error",
        },
    },
    "zh-CN": {
        editor: {
            copyCode: "复制代码",
            moreActions: "更多操作",
            noActionsAvailable: "暂无可用操作",
            mermaidError: "Mermaid 错误",
        },
    },
};

function currentLocale() {
    if (typeof window !== "undefined") {
        const locale = window.deve_i18n?.locale;
        if (typeof locale === "string" && locale.startsWith("zh")) {
            return "zh-CN";
        }
    }
    return DEFAULT_LOCALE;
}

export function editorCopy(key) {
    if (typeof window !== "undefined") {
        const value = window.deve_i18n?.editor?.[key];
        if (typeof value === "string" && value.length > 0) {
            return value;
        }
    }

    const locale = currentLocale();
    return FALLBACK_COPY[locale]?.editor?.[key]
        ?? FALLBACK_COPY[DEFAULT_LOCALE].editor[key]
        ?? key;
}
