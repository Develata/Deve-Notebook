export class CdpPage {
  constructor(socket, withDeadline) {
    this.socket = socket;
    this.withDeadline = withDeadline;
    this.nextId = 1;
    this.pending = new Map();
    socket.addEventListener("message", (event) => {
      const message = JSON.parse(String(event.data));
      if (message.id === undefined) return;
      const waiter = this.pending.get(message.id);
      if (!waiter) return;
      this.pending.delete(message.id);
      if (message.error) waiter.reject(new Error(`${waiter.method}: ${message.error.message}`));
      else waiter.resolve(message.result ?? {});
    });
    socket.addEventListener("close", () => {
      for (const waiter of this.pending.values()) {
        waiter.reject(new Error(`CDP socket closed during ${waiter.method}`));
      }
      this.pending.clear();
    });
  }

  static async connect(webSocketDebuggerUrl, withDeadline) {
    const socket = new WebSocket(webSocketDebuggerUrl);
    try {
      await withDeadline("Android WebView CDP socket open", new Promise((resolve, reject) => {
        socket.addEventListener("open", resolve, { once: true });
        socket.addEventListener("error", () => reject(new Error("Android WebView CDP socket failed")), { once: true });
      }), 10000);
      const page = new CdpPage(socket, withDeadline);
      await page.send("Runtime.enable");
      return page;
    } catch (error) {
      try {
        socket.close();
      } catch {}
      throw error;
    }
  }

  send(method, params = {}) {
    const id = this.nextId++;
    return this.withDeadline(method, new Promise((resolve, reject) => {
      this.pending.set(id, { method, resolve, reject });
      this.socket.send(JSON.stringify({ id, method, params }));
    }));
  }

  async evaluate(expression) {
    const response = await this.send("Runtime.evaluate", {
      expression,
      awaitPromise: true,
      returnByValue: true,
      userGesture: true,
    });
    if (response.exceptionDetails) {
      const description = response.exceptionDetails.exception?.description
        ?? response.exceptionDetails.text
        ?? "JavaScript evaluation failed";
      throw new Error(description);
    }
    return response.result?.value;
  }

  call(fn, ...args) {
    return this.evaluate(`(${fn.toString()})(...${JSON.stringify(args)})`);
  }

  async close() {
    if (this.socket.readyState === WebSocket.CLOSED) return;
    this.socket.close();
    await this.withDeadline("Android WebView CDP socket close", new Promise((resolve) => {
      this.socket.addEventListener("close", resolve, { once: true });
    }), 2000).catch(() => {});
  }
}

export function visibleElement(selector) {
  return [...document.querySelectorAll(selector)].find((element) => {
    const style = getComputedStyle(element);
    const rect = element.getBoundingClientRect();
    return style.display !== "none" && style.visibility !== "hidden" && rect.width > 0 && rect.height > 0;
  }) ?? null;
}

export async function findStableAppPage({ cdpEndpoint, withDeadline, waitUntil }) {
  const listTargets = async () => {
    const response = await withDeadline(
      "Android WebView target discovery",
      fetch(`${cdpEndpoint}/json`),
      10000,
    );
    if (!response.ok) throw new Error(`CDP target discovery returned ${response.status}`);
    return response.json();
  };

  const findAppPage = async () => {
    const targets = await listTargets();
    const discoveredTargets = targets.map(({ type, title, url }) => ({ type, title, url }));
    const target = targets.find((candidate) =>
      candidate.webSocketDebuggerUrl
      && candidate.type === "page"
      && candidate.url === "http://tauri.localhost/");
    if (!target) {
      throw new Error(`Android WebView target unavailable; targets=${JSON.stringify(discoveredTargets)}`);
    }
    console.log(`mobile-android-lifecycle: attaching page CDP ${target.title}`);
    let page;
    try {
      page = await CdpPage.connect(target.webSocketDebuggerUrl, withDeadline);
      console.log("mobile-android-lifecycle: page CDP attached");
      await waitUntil("Android app DOM", () => page.call(() =>
        Boolean(document.querySelector("[data-deve-sync-status]"))), 10000);
      await page.evaluate(`globalThis.__deveVisibleElement = ${visibleElement.toString()}`);
    } catch (error) {
      const diagnostics = page
        ? await page.call(() => ({
          url: location.href,
          readyState: document.readyState,
          title: document.title,
          bodyText: (document.body?.textContent ?? "").slice(0, 500),
          bodyHtml: (document.body?.innerHTML ?? "").slice(0, 500),
        })).catch((diagnosticError) => ({ diagnosticError: diagnosticError.message }))
        : { diagnosticError: "page unavailable before Runtime.enable" };
      await page?.close();
      throw new Error(`${error.message}; page=${JSON.stringify(diagnostics)}`);
    }
    return page;
  };

  return waitUntil("stable Android WebView page", async () => {
    try {
      return await findAppPage();
    } catch (error) {
      const message = String(error?.message ?? error);
      if (message.includes("Inspected target navigated or closed")
        || message.includes("CDP socket closed")
        || message.includes("Android WebView target unavailable")) {
        console.log(`mobile-android-lifecycle: retrying page CDP after navigation: ${message}`);
        return null;
      }
      throw error;
    }
  }, 60000);
}
