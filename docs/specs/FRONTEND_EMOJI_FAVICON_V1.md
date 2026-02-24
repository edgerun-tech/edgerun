# Frontend Emoji Favicon V1

## Goal
- Replace dynamic raster/favicon badge rendering with emoji-based dynamic favicons.
- Preserve existing status signaling (neutral/running/success/warning/error) while using emoji glyphs.

## Non-goals
- No changes to title text logic.
- No changes to job/terminal/wallet status sources.
- No changes to static PWA icon manifest assets.

## Security and constraints
- Keep deterministic in-browser rendering without external icon fetches.
- Do not add runtime dependencies.
- Continue using the existing `data-edgerun-dynamic-favicon` link element.
- Generate XML-safe SVG text content for favicon glyphs.
- Respect reduced-motion user preference by disabling favicon frame animation when `prefers-reduced-motion: reduce` is active.

## Acceptance criteria
- Dynamic favicon uses `image/svg+xml` data URL with emoji text glyphs.
- Running state remains animated via frame updates.
- Existing page chrome status flow remains intact.
- Cypress test asserts dynamic favicon is present and emoji-based SVG data URL is used.

## Rollout and rollback
- Rollout: update `frontend/src/client.tsx` favicon generator and add/adjust Cypress assertion.
- Rollback: restore prior canvas/png favicon renderer if emoji rendering proves inconsistent.
