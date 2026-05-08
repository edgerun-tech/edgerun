# Sections

Section components are reusable chunks of app content.

Use this folder for:

- metric cards
- resource summaries
- finance chart sections
- node lists
- trust detail panels
- reusable dashboard slices

Rules:

- Sections can contain domain UI, but should not own full app navigation or desktop placement.
- If there are multiple versions of the same section, consolidate them into one plural file with named exports.
- Do not default-export section variants.
- Promote a section into `components/apps` only when it becomes a full app surface.
