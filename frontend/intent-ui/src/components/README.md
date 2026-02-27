# Solid Components Bundle

This directory is now organized by feature so components are easier to browse and wire into a showcase app.

## Structure

- `apps/`: app-like full-screen experiences
- `layout/`: windowing/shell primitives
- `onboarding/`: setup and onboarding flows
- `panels/`: dockable/standalone panel components
- `results/`: result renderer and result view components
- `ui/`: shared low-level UI primitives

## Consumption

Use `components/index.ts` as the top-level barrel export, or import directly from feature folders for smaller scoped views.
