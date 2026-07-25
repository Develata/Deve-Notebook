const defaultTimeoutMs = Number(
  process.env.DEVE_REMOTE_IMPORT_BROWSER_CLOSE_TIMEOUT_MS ?? "15000",
);

async function closeWithin(label, resource, timeoutMs) {
  if (!resource) return;
  let timer;
  try {
    await Promise.race([
      resource.close(),
      new Promise((_, reject) => {
        timer = setTimeout(
          () => reject(new Error(`${label} close timed out after ${timeoutMs}ms`)),
          timeoutMs,
        );
      }),
    ]);
  } finally {
    if (timer) clearTimeout(timer);
  }
}

export async function closeBrowserResources(
  context,
  browser,
  timeoutMs = defaultTimeoutMs,
) {
  const failures = [];
  for (const [label, resource] of [
    ["Playwright context", context],
    ["Playwright browser", browser],
  ]) {
    try {
      await closeWithin(label, resource, timeoutMs);
    } catch (error) {
      failures.push(error);
    }
  }
  if (failures.length > 0) {
    throw new AggregateError(
      failures,
      "Remote Import browser resources did not close cleanly",
    );
  }
}
