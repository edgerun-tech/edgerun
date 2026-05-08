# Apps

App components are full product surfaces: Settings, Finances, Trust Manager, People, Terminal, Files, Gmail, and similar user-facing apps.

Rules:

- Keep one canonical implementation per app.
- If two app implementations exist, keep the more mature one and remove or absorb the weaker one.
- App IDs should match product concepts: `finances`, `people`, `settings`, not old implementation names like `wallet`.
- Apps may use layouts, sections, and UI primitives, but should not own desktop placement.
- Desktop placement belongs to `components/layouts` and `stores/desktop-store.ts` app surfaces.

Current migration target:

- Move mature app surfaces out of `components/os` into this folder over time.
- Keep `components/os` as shell/runtime glue only.
