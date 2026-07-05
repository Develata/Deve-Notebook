// index_theme_bootstrap.js
// Pre-paint theme marker bootstrap. This stays outside index.html so the HTML
// shell only owns resource loading order.

(function () {
  const normalizeThemePref = (value) => {
    switch (value) {
      case "warm":
      case "cold":
      case "night":
        return value;
      case "dark":
        return "night";
      case "auto":
      case "light":
        return "warm";
      default:
        return "warm";
    }
  };

  // Always set the marker pre-paint (default warm) so the first paint carries
  // data-deve-theme-pref and its color-scheme even before any theme is stored.
  let stored = null;
  try {
    stored = globalThis.localStorage?.getItem("deve.ui.theme");
  } catch (_) {
    // localStorage can be unavailable in private or restricted contexts.
  }
  document.documentElement.setAttribute(
    "data-deve-theme-pref",
    normalizeThemePref(stored),
  );
})();
