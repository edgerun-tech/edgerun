# Control Panel Frontend (WIP)

This directory is the starting point for moving UI out of `crates/edgerun-cli` and into a dedicated frontend app.

## Structure

- `index.html`: shell layout
- `src/main.js`: dashboard bootstrapping
- `src/components/terminalDrawer.js`: global terminal drawer UI
- `src/state/store.js`: tiny reactive store helper
- `src/state/indexedDbAdapter.js`: IndexedDB-backed persistence with localStorage fallback
- `src/services/api.js`: API calls
- `src/styles.css`: app styling

## Notes

- Drawer state persists via IndexedDB (`edgerun-control-ui/ui_state`).
- Drawer still uses iframe panes pointed at `/term/` by default.
- This is framework-agnostic vanilla JS for now, intended as a cleanup baseline before selecting a full frontend stack.
