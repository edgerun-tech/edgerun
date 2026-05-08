# Layouts

Layout components own placement and composition, not domain content.

Use this folder for:

- desktop frames
- overlay hosts
- slot layouts
- app shell wrappers
- split-panel/grid composition

Rules:

- Layouts may accept app/content components as children.
- Layouts should not contain business logic, provider APIs, exchange logic, auth logic, or app-specific state.
- If there are multiple layout variants, keep them in one plural file with named exports.
- Do not default-export layout variants.
