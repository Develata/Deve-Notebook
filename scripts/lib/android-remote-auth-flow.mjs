export function readRemoteEntryState(expectedOrigin) {
  let requiredOrigin;
  try {
    requiredOrigin = new URL(expectedOrigin).origin;
  } catch {
    return { kind: "invalid-origin" };
  }
  if (location.origin !== requiredOrigin) return { kind: "unexpected-origin" };
  const status = document.querySelector("[data-deve-sync-status]")
    ?.getAttribute("data-deve-sync-status");
  if (status === "ready") return { kind: "ready" };
  const username = globalThis.__deveVisibleElement("#login-username");
  const password = globalThis.__deveVisibleElement("#login-password");
  const submit = globalThis.__deveVisibleElement('button[type="submit"]');
  return username && password && submit && !submit.disabled
    ? { kind: "login" }
    : null;
}

export function fillRemoteLoginCredentials(expectedOrigin, username, password) {
  let requiredOrigin;
  try {
    requiredOrigin = new URL(expectedOrigin).origin;
  } catch {
    return { kind: "invalid-origin" };
  }
  if (location.origin !== requiredOrigin) return { kind: "unexpected-origin" };
  const usernameInput = globalThis.__deveVisibleElement("#login-username");
  const passwordInput = globalThis.__deveVisibleElement("#login-password");
  if (!(usernameInput instanceof HTMLInputElement)
    || !(passwordInput instanceof HTMLInputElement)) {
    return { kind: "login-unavailable" };
  }
  for (const [element, value] of [[usernameInput, username], [passwordInput, password]]) {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value").set.call(element, value);
    element.dispatchEvent(new InputEvent("input", {
      bubbles: true,
      inputType: "insertText",
      data: value,
    }));
    element.dispatchEvent(new Event("change", { bubbles: true }));
  }
  return { kind: "credentials-filled" };
}

export function submitRemoteLogin(expectedOrigin) {
  let requiredOrigin;
  try {
    requiredOrigin = new URL(expectedOrigin).origin;
  } catch {
    return { kind: "invalid-origin" };
  }
  if (location.origin !== requiredOrigin) return { kind: "unexpected-origin" };
  const submit = globalThis.__deveVisibleElement('button[type="submit"]');
  if (!submit || submit.disabled) return { kind: "login-unavailable" };
  submit.click();
  return { kind: "submitted" };
}

export function readRemoteReadyState(expectedOrigin) {
  let requiredOrigin;
  try {
    requiredOrigin = new URL(expectedOrigin).origin;
  } catch {
    return { kind: "invalid-origin" };
  }
  if (location.origin !== requiredOrigin) return { kind: "unexpected-origin" };
  return document.querySelector("[data-deve-sync-status]")
    ?.getAttribute("data-deve-sync-status") === "ready"
    ? { kind: "ready" }
    : null;
}

export async function loginAndroidRemote(
  page,
  expectedOrigin,
  username,
  password,
  waitUntil,
) {
  const entry = await waitUntil("remote Android entry state", () =>
    page.call(readRemoteEntryState, expectedOrigin), 60000);
  if (entry.kind === "unexpected-origin" || entry.kind === "invalid-origin") {
    throw new Error(`remote Android entry rejected: ${entry.kind}`);
  }
  if (entry.kind === "login") {
    const filled = await page.call(
      fillRemoteLoginCredentials,
      expectedOrigin,
      username,
      password,
    );
    if (filled.kind !== "credentials-filled") {
      throw new Error(`remote Android credentials rejected: ${filled.kind}`);
    }
    const submitted = await page.call(submitRemoteLogin, expectedOrigin);
    if (submitted.kind !== "submitted") {
      throw new Error(`remote Android login submit rejected: ${submitted.kind}`);
    }
  }
  const ready = await waitUntil("remote Android ready", () =>
    page.call(readRemoteReadyState, expectedOrigin), 60000);
  if (ready.kind !== "ready") {
    throw new Error(`remote Android ready rejected: ${ready.kind}`);
  }
}
