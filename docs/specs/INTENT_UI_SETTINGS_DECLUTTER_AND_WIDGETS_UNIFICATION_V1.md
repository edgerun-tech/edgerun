# INTENT_UI_SETTINGS_DECLUTTER_AND_WIDGETS_UNIFICATION_V1

## Goal
Redesign the Intent UI settings surface to reduce visual clutter and make wallpaper widget controls available directly in Settings instead of requiring a separate Widgets window.

## Non-Goals
- Reworking wallpaper widget rendering/drag behavior.
- Changing preference storage keys or persistence format.
- Removing existing `widgets` window id support from the window store/event intents.

## Security / Constraint Requirements
- Preserve existing local preference persistence semantics (`intent-ui-preferences-v1`).
- Keep controls deterministic and fail-closed when browser storage is unavailable.
- Maintain current architecture: local bridge requirements and profile gating behavior remain unchanged.
- Do not introduce new package dependencies.

## Acceptance Criteria
1. Settings panel uses a clearer grouped layout (overview/header + separated sections).
2. Wallpaper widget toggles and widget-position reset actions are directly available inside Settings.
3. Settings no longer asks users to open a separate Widgets panel to manage wallpaper widgets.
4. Any entry points that currently open `widgets` should route to the Settings experience.
5. Existing widget toggles (`map`, `clock`, `weather`, `bookmarks`) still update `preferences().wallpaperWidgets`.
6. Frontend validation passes:
   - `cd frontend && bun run check`
   - `cd frontend && bun run build`
7. UI regression coverage is updated with a Cypress spec that proves widgets are configurable from Settings.

## Rollout Notes
- Ship as a UI-only frontend change.
- Preserve fallback handling so legacy calls to `openWindow("widgets")` continue to work by showing Settings content.

## Rollback Notes
- Revert settings panel/layout changes and route mappings that redirect widgets to settings.
- Restore prior dedicated widgets-launch behavior in quick actions/context menus.
