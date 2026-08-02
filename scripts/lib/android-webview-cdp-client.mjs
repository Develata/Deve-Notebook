export const DISCOVERY_COMMAND_TIMEOUT_MS = 10_000;

export function isExpectedCdpTargetRetirement(error) {
  const message = String(error?.message ?? error);
  return /CDP socket closed during Runtime\.evaluate|Inspected target navigated or closed/i
    .test(message);
}

export class CdpPage {
  constructor(socket, withDeadline) {
    this.socket = socket;
    this.withDeadline = withDeadline;
    this.nextId = 1;
    this.pending = new Map();
    this.listeners = new Map();
    socket.addEventListener("message", (event) => {
      const message = JSON.parse(String(event.data));
      if (message.id === undefined) {
        for (const listener of this.listeners.get(message.method) ?? []) {
          listener(message.params ?? {});
        }
        return;
      }
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

  static async connect(
    webSocketDebuggerUrl,
    withDeadline,
    commandBudget = () => DISCOVERY_COMMAND_TIMEOUT_MS,
  ) {
    const socket = new WebSocket(webSocketDebuggerUrl);
    try {
      await withDeadline("Android WebView CDP socket open", new Promise((resolve, reject) => {
        socket.addEventListener("open", resolve, { once: true });
        socket.addEventListener("error", () => reject(
          new Error("Android WebView CDP socket failed"),
        ), { once: true });
      }), commandBudget());
      const page = new CdpPage(socket, withDeadline);
      await page.send("Runtime.enable", {}, commandBudget());
      return page;
    } catch (error) {
      try {
        socket.close();
      } catch {}
      throw error;
    }
  }

  async send(method, params = {}, timeoutMs) {
    const id = this.nextId++;
    const command = new Promise((resolve, reject) => {
      this.pending.set(id, { method, resolve, reject });
      this.socket.send(JSON.stringify({ id, method, params }));
    });
    try {
      return await this.withDeadline(method, command, timeoutMs);
    } finally {
      this.pending.delete(id);
    }
  }

  on(method, listener) {
    const listeners = this.listeners.get(method) ?? [];
    listeners.push(listener);
    this.listeners.set(method, listeners);
  }

  async evaluate(expression, timeoutMs) {
    const response = await this.send("Runtime.evaluate", {
      expression,
      awaitPromise: true,
      returnByValue: true,
      userGesture: true,
    }, timeoutMs);
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

  callWithin(timeoutMs, fn, ...args) {
    return this.evaluate(
      `(${fn.toString()})(...${JSON.stringify(args)})`,
      timeoutMs,
    );
  }

  async close() {
    if (this.socket.readyState === 3) return;
    const closed = new Promise((resolve) => {
      this.socket.addEventListener("close", resolve, { once: true });
    });
    try {
      if (this.socket.readyState !== 2) this.socket.close();
    } catch {
      return;
    }
    await this.withDeadline("Android WebView CDP socket close", closed, 2000).catch(() => {});
  }
}

export function visibleElement(selector) {
  return [...document.querySelectorAll(selector)].find((element) => {
    const style = getComputedStyle(element);
    const rect = element.getBoundingClientRect();
    return style.display !== "none" && style.visibility !== "hidden" && rect.width > 0 && rect.height > 0;
  }) ?? null;
}
