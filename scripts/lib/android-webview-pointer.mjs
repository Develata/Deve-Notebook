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
