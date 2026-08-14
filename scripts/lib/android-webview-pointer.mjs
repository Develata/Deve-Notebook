export async function clickWebViewPoint(page, point, { beforePress } = {}) {
  if (!Number.isFinite(point?.x) || !Number.isFinite(point?.y)) {
    throw new Error("pointer admission received an invalid initial point");
  }
  await page.send("Input.dispatchMouseEvent", {
    type: "mouseMoved",
    x: point.x,
    y: point.y,
    button: "none",
    buttons: 0,
  });
  const pressPoint = beforePress ? (await beforePress()) ?? point : point;
  if (!Number.isFinite(pressPoint?.x) || !Number.isFinite(pressPoint?.y)) {
    throw new Error("before-press pointer admission returned an invalid point");
  }
  await page.send("Input.dispatchMouseEvent", {
    type: "mousePressed",
    x: pressPoint.x,
    y: pressPoint.y,
    button: "left",
    buttons: 1,
    clickCount: 1,
    force: 0.5,
  });
  await page.send("Input.dispatchMouseEvent", {
    type: "mouseReleased",
    x: pressPoint.x,
    y: pressPoint.y,
    button: "left",
    buttons: 0,
    clickCount: 1,
  });
}

const NATIVE_TAP_CONTACT_MS = 50;

function waitForNativeTapContact(durationMs) {
  return new Promise((resolve) => setTimeout(resolve, durationMs));
}

export async function tapWebViewPoint(
  page,
  point,
  {
    beforeContact,
    waitForContact = waitForNativeTapContact,
  } = {},
) {
  if (!Number.isFinite(point?.x) || !Number.isFinite(point?.y)) {
    throw new Error("pointer admission received an invalid initial point");
  }
  const contactPoint = beforeContact ? (await beforeContact()) ?? point : point;
  if (!Number.isFinite(contactPoint?.x) || !Number.isFinite(contactPoint?.y)) {
    throw new Error("before-contact pointer admission returned an invalid point");
  }
  try {
    await page.send("Input.dispatchTouchEvent", {
      type: "touchStart",
      touchPoints: [{
        x: contactPoint.x,
        y: contactPoint.y,
        radiusX: 1,
        radiusY: 1,
        force: 1,
        id: 0,
      }],
    });
    await waitForContact(NATIVE_TAP_CONTACT_MS);
    await page.send("Input.dispatchTouchEvent", {
      type: "touchEnd",
      touchPoints: [],
    });
  } catch (error) {
    let cancelOutcome = "confirmed";
    try {
      await page.send("Input.dispatchTouchEvent", {
        type: "touchCancel",
        touchPoints: [],
      });
    } catch {
      cancelOutcome = "failed";
    }
    throw new Error(`${error?.message ?? String(error)}; touch_cancel=${cancelOutcome}`);
  }
}
