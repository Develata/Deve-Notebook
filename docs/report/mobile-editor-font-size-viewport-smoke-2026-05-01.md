# Mobile Editor Font Size Viewport Smoke 2026-05-01

This report closes the `Mobile editor 16px viewport smoke` active queue item.

## Result

- Blocking failures: 0.
- Chrome MCP 375x812 mobile viewport smoke confirmed `.cm-content` is mounted in
  the real Web shell and its computed `font-size` is `16px`.
- Focusing `.cm-content` keeps the computed `font-size` at `16px`.
- `visualViewport.scale` remained `1` during the smoke.
- No browser console warning/error was observed during the smoke.

## Chrome MCP Result

- URL: `http://127.0.0.1:8080/`.
- Viewport emulation: `375x812`, `isMobile=true`, `hasTouch=true`,
  `deviceScaleFactor=2`.
- Fixture: isolated `/tmp/deve-mobile-font-smoke` ledger/vault, dev auth.
- Created document: `Untitled.md`.
- `.cm-content`: exists.
- `.cm-editor`: exists.
- Computed `.cm-content font-size`: `16px`.
- Focused `.cm-content font-size`: `16px`.
- Focused element: `DIV.cm-content.cm-lineWrapping`.
- `visualViewport`: `width=375`, `height=812`, `scale=1`.
- `.cm-editor` rect: `x=0,y=49,w=374,h=712`.
- `.cm-content` rect: `x=21,y=49,w=353,h=712`.

## Verified

```bash
DEVE_LEDGER_DIR=/tmp/deve-mobile-font-smoke/ledger DEVE_VAULT_PATH=/tmp/deve-mobile-font-smoke/vault cargo run -p deve_cli --bin deve_cli -- serve --dev --port 3001
NO_COLOR=true trunk serve --address 127.0.0.1 --port 8080
scripts/check-mobile-baseline.sh
```

Chrome MCP checks:

```javascript
getComputedStyle(document.querySelector(".cm-content")).fontSize === "16px"
document.querySelector(".cm-content").focus()
window.visualViewport.scale === 1
```

## Next Narrow Batch

Run a post-mobile-baseline priority reselection pass before opening the next
implementation domain.
