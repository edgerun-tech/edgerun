# EdgeRun UI Component Roadmap

This tracks the shared Rust UI kit used by native, SDL/OpenGL, and WASM/WebGL
hosts. Components should render into `GpuScene` and avoid browser/DOM-specific
behavior.

## 1. Foundation Controls

- [x] Checkbox
- [x] Radio
- [x] Select/dropdown trigger
- [x] Tooltip helper

## 2. Feedback And System Surfaces

- [x] Modal/dialog
- [x] Toast
- [x] Empty state
- [x] Loading/skeleton
- [x] Spinner/progress ring

## 3. Data And Navigation

- [x] Table
- [x] Breadcrumb
- [x] Command palette
- [x] Tree view
- [x] Section/list grouping

## 4. EdgeRun Domain Components

- [x] Identity card
- [x] Contact card
- [x] Thread row
- [x] Message attachment/media preview
- [x] Capability grant row
- [x] Proof/audit event row
- [x] Route/relay path visual
- [x] Package/app card
- [x] Receipt/payment row

## 5. System Behavior

- [x] Consistent disabled/loading state
- [x] Keyboard navigation beyond text fields
- [x] Icon atlas path for GPU hosts
- [x] Component gallery/examples

## 6. Integration

- [x] Use reusable domain components in Trust Manager preview
- [x] Use reusable domain components in Storage preview
- [x] Use reusable controls in lock/capability system screens
