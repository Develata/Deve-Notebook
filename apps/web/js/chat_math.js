// apps/web/js/chat_math.js
// Lightweight KaTeX post-render pass for AI chat Markdown message bodies.

(function () {
  const SKIP_TAGS = new Set(["CODE", "PRE", "SCRIPT", "STYLE", "TEXTAREA", "OPTION"]);
  const SKIP_SELECTOR = ".katex, .katex-display, .chat-math, .markdown-code-block";

  function isEscaped(text, index) {
    let slashes = 0;
    for (let i = index - 1; i >= 0 && text[i] === "\\"; i--) {
      slashes++;
    }
    return slashes % 2 === 1;
  }

  function findBlockClose(text, start) {
    for (let i = start; i < text.length - 1; i++) {
      if (text.startsWith("$$", i) && !isEscaped(text, i)) {
        return i;
      }
    }
    return -1;
  }

  function findInlineClose(text, start) {
    for (let i = start; i < text.length; i++) {
      const c = text[i];
      if (c === "\\" && i + 1 < text.length) {
        i++;
        continue;
      }
      if (c === "\n" && text[i + 1] === "\n") {
        return -1;
      }
      if (c === "$" && !isEscaped(text, i) && !/\s/.test(text[i - 1] || "")) {
        return i;
      }
    }
    return -1;
  }

  function splitMath(text) {
    const parts = [];
    let i = 0;
    let textStart = 0;

    while (i < text.length) {
      if (text[i] !== "$" || isEscaped(text, i)) {
        i++;
        continue;
      }

      if (text.startsWith("$$", i)) {
        const close = findBlockClose(text, i + 2);
        if (close === -1) {
          i += 2;
          continue;
        }
        if (textStart < i) {
          parts.push({ type: "text", value: text.slice(textStart, i) });
        }
        parts.push({
          type: "math",
          value: text.slice(i + 2, close),
          raw: text.slice(i, close + 2),
          display: true,
        });
        i = close + 2;
        textStart = i;
        continue;
      }

      const next = text[i + 1];
      if (!next || /\s/.test(next)) {
        i++;
        continue;
      }

      const close = findInlineClose(text, i + 1);
      if (close === -1) {
        i++;
        continue;
      }
      if (textStart < i) {
        parts.push({ type: "text", value: text.slice(textStart, i) });
      }
      parts.push({
        type: "math",
        value: text.slice(i + 1, close),
        raw: text.slice(i, close + 1),
        display: false,
      });
      i = close + 1;
      textStart = i;
    }

    if (textStart < text.length) {
      parts.push({ type: "text", value: text.slice(textStart) });
    }
    return parts;
  }

  function shouldSkipNode(node) {
    const parent = node.parentElement;
    if (!parent) return true;
    if (SKIP_TAGS.has(parent.tagName)) return true;
    return Boolean(parent.closest(SKIP_SELECTOR));
  }

  function renderMathNode(part) {
    const wrapper = document.createElement("span");
    wrapper.className = part.display ? "chat-math chat-math-block" : "chat-math";

    const katex = window.katex;
    if (!katex || typeof katex.render !== "function") {
      wrapper.textContent = part.raw;
      return wrapper;
    }

    try {
      katex.render(part.value, wrapper, {
        displayMode: part.display,
        throwOnError: false,
        trust: false,
      });
    } catch (_err) {
      wrapper.textContent = part.raw;
    }

    return wrapper;
  }

  function replaceTextNode(node) {
    const text = node.nodeValue || "";
    if (!text.includes("$") || shouldSkipNode(node)) {
      return;
    }

    const parts = splitMath(text);
    if (!parts.some((part) => part.type === "math")) {
      return;
    }

    const fragment = document.createDocumentFragment();
    for (const part of parts) {
      if (part.type === "text") {
        fragment.appendChild(document.createTextNode(part.value));
      } else {
        fragment.appendChild(renderMathNode(part));
      }
    }
    node.parentNode.replaceChild(fragment, node);
  }

  function renderChatMath(root) {
    if (!root) return false;

    const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
    const nodes = [];
    while (walker.nextNode()) {
      nodes.push(walker.currentNode);
    }

    for (const node of nodes) {
      replaceTextNode(node);
    }
    return true;
  }

  const bridge = window.__deveWebBridge;
  if (bridge && typeof bridge.register === "function") {
    bridge.register("renderChatMath", renderChatMath, {
      runtime: "rendering_client",
      boundary: "object-plane-adapter",
    });
    bridge.register("__deveChatMath", { splitMath }, {
      runtime: "rendering_client",
      boundary: "test-support",
    });
  } else {
    window.renderChatMath = renderChatMath;
    window.__deveChatMath = { splitMath };
  }
})();
