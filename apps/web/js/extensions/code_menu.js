/**
 * Code Menu Module (代码块菜单)
 * 
 * Provides dynamic menu rendering for code toolbar actions.
 * Supports plugin registration via `window.deve_code_actions`.
 * 
 * Action Protocol:
 * ```js
 * window.deve_code_actions.push({
 *   id: 'unique-id',
 *   label: 'Display Label',
 *   run: ({ code, language, view }) => { ... }
 * });
 * ```
 */

// Initialize global registry
if (typeof window !== 'undefined') {
    window.deve_code_actions = window.deve_code_actions || [];
}

let activeAnchor = null;

/**
 * Get registered actions (safe access)
 * @returns {Array} Array of action objects
 */
export function getActions() {
    return (typeof window !== 'undefined' && Array.isArray(window.deve_code_actions))
        ? window.deve_code_actions
        : [];
}

/**
 * Toggle the action menu (show if hidden, hide if shown)
 */
export function toggleMenu(anchor, context) {
    if (activeAnchor === anchor && document.getElementById("deve-code-menu")) {
        closeMenu();
        return;
    }
    showMenu(anchor, context);
}

/**
 * Create and show the action menu
 * @param {HTMLElement} anchor - Button to anchor the menu to
 * @param {Object} context - { code, language, view }
 * @returns {HTMLElement} The menu element
 */
export function showMenu(anchor, context) {
    closeMenu();
    activeAnchor = anchor;
    
    const actions = getActions();
    const menu = document.createElement("div");
    menu.id = "deve-code-menu";
    menu.className = "absolute top-full right-0 mt-1 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 shadow-lg rounded py-1 text-xs z-50 min-w-[120px] flex flex-col";
    
    if (actions.length === 0) {
        const empty = document.createElement("div");
        empty.className = "px-3 py-2 text-gray-400 dark:text-gray-500 whitespace-nowrap";
        empty.textContent = "No actions available";
        menu.appendChild(empty);
    } else {
        for (const action of actions) {
            const item = document.createElement("button");
            item.className = "block w-full px-3 py-1.5 text-left hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-700 dark:text-gray-200 transition-colors whitespace-nowrap";
            item.textContent = action.label || action.id;
            item.onclick = (e) => {
                e.preventDefault();
                e.stopPropagation();
                closeMenu();
                try {
                    action.run(context);
                } catch (err) {
                    console.error(`[deve] Action "${action.id}" failed:`, err);
                }
            };
            menu.appendChild(item);
        }
    }
    
    anchor.appendChild(menu);
    setTimeout(() => {
        document.addEventListener("click", handleClickOutside, { capture: true });
    }, 0);
    return menu;
}

/**
 * Close the active menu
 */
export function closeMenu() {
    const existing = document.getElementById("deve-code-menu");
    if (existing) existing.remove();
    activeAnchor = null;
    document.removeEventListener("click", handleClickOutside, { capture: true });
}

/** Handle click outside menu (excludes anchor to allow toggle) */
function handleClickOutside(e) {
    if (activeAnchor && activeAnchor.contains(e.target)) return;
    const menu = document.getElementById("deve-code-menu");
    if (menu && !menu.contains(e.target)) {
        closeMenu();
    }
}
