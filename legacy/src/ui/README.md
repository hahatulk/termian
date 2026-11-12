# UI

The Windows/macOS Termian frontend, shipped as prebuilt React/Redux bundles.

## Entry points

- [index.html](index.html) — main window.
- [settings.html](settings.html) — settings window.
- [background.html](background.html) — hidden desktop Vault process.
- [assets/ui-process-62a480c2.js](assets/ui-process-62a480c2.js) — main React renderer.
- [assets/reconnectSaga-3b11f62e.js](assets/reconnectSaga-3b11f62e.js) — shared UI modules and state.
- [../original-bridge.js](../original-bridge.js) — maintained Tauri compatibility bridge.

[symbols.json](symbols.json) indexes declarations; [inventory.json](inventory.json) records bundled assets.

## Editing

Run from `legacy/`:

```bash
npm run index:ui
npm run check:web
npm test
```

Edit files in `src/ui/`, not the generated `dist/web/`. Keep hashed asset filenames and their imports consistent. Some bundles remain large.


See the [Tauri client README](../../README.md) and [repository rules](../../../AGENTS.md).
