# Frontend Blog SEO and A11y Foundation V1

## Goal
- Remove placeholder blog UX and publish a readable "why this project exists" narrative post.
- Improve foundational SEO metadata for static pages.
- Strengthen baseline a11y semantics for blog reading experience.

## Non-goals
- No CMS or dynamic publishing backend.
- No exhaustive WCAG audit across every page in this change.

## Security and constraints
- Keep blog content deterministic and in-repo.
- Preserve static generation workflow.
- Avoid placeholder/"Generating" copy on blog index and blog post surfaces.

## Acceptance criteria
1. `/blog/` contains no placeholder-generating cards and presents actionable post links.
2. `/blog/<slug>/` renders long-form structured content explaining project purpose.
3. Static page output includes canonical + OG + Twitter meta fields.
4. Cypress coverage verifies blog page readability/metadata and no placeholder copy.
5. `cd frontend && bun run check`, `cd frontend && bun run build`, `cd frontend && bun run e2e:core` pass.

## Rollout and rollback
- Rollout: ship content + metadata + tests together.
- Rollback: revert this changeset.
