export async function clickAndroidNewDocumentActionWhenAdmitted() {
  const observe = () => {
    const action = globalThis.__deveVisibleElement("[data-deve-new-doc-button=true]");
    const mobile = Boolean(globalThis.__deveVisibleElement('[data-deve-layout-mode="mobile"]'));
    const drawerOpen = document.querySelector('[data-deve-mobile-drawer="left"]')
      ?.getAttribute("data-deve-mobile-drawer-open") === "true";
    const explorerActive = document.querySelector('[data-deve-mobile-sidebar-tab="explorer"]')
      ?.getAttribute("data-deve-mobile-sidebar-tab-active") === "true";
    return { action, mobile, drawerOpen, explorerActive };
  };
  const before = observe();
  await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
  const after = observe();
  const ready = Boolean(after.action)
    && before.action === after.action
    && (!after.mobile || (after.drawerOpen && after.explorerActive));
  if (ready) after.action.click();
  return {
    clicked: ready,
    mobile: after.mobile,
    drawerOpen: after.drawerOpen,
    explorerActive: after.explorerActive,
  };
}
