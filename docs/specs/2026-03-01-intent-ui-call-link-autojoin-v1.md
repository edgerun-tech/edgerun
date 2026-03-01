# 2026-03-01 Intent UI Call Link Auto-Join V1

## Goal and non-goals
- Goal:
  - Make shared call links join immediately after page load, without requiring manual ID paste or extra click.
  - Keep call setup direct peer-to-peer from frontend (no media relay backend).
  - Simplify primary UX around link sharing by keeping a single `Copy Link` action.
  - Add one-click call launch from IntentBar that auto-copies share link when peer identity is ready.
  - Surface pending call links as persistent conversation threads, sorted to top by recency.
- Non-goals:
  - Introduce new signaling infrastructure in this change.
  - Add TURN/STUN reconfiguration flows.

## Security and constraint requirements
- Do not persist camera/microphone streams beyond active call session.
- Keep shared link handling bounded to explicit `/call/<peerId>` paths.
- Maintain existing local cleanup behavior on window/component unmount.

## Acceptance criteria
1. Visiting a URL containing `/call/<peerId>` opens the Call window automatically.
2. Call app parses the `<peerId>` from URL and auto-starts connection when local peer is ready.
3. The previous random `New Call ID` action is removed from the call UI.
4. `Copy Link` remains available and points to `/call/<localPeerId>`.
5. IntentBar includes a single-click call button that opens Call Studio and triggers link copy when the peer is ready.
6. Call link copy emits a persistent pending thread entry (`call-link-*`) visible in conversations and retained across reload via local conversation storage.
7. Call controls include microphone mute/unmute and fullscreen toggle actions.
8. Add/update Cypress coverage that verifies auto-open + auto-join behavior with mocked peer/media dependencies.
9. Frontend validation passes:
- `cd frontend && bun run check`
- `cd frontend && bun run build`

## Rollout and rollback
- Rollout:
  - Auto-open Call window when route includes `/call/`.
  - Auto-join from URL target inside Call app.
  - Add IntentBar one-click call launch event.
  - Persist pending call threads in conversations.
  - Add Cypress regression coverage for shared-link behavior.
- Rollback:
  - Revert call window route auto-open, call auto-join logic, intentbar quick-call event, pending thread persistence, and call link Cypress spec.
