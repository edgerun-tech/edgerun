# Frontend Legal Pages Generated Content V1

## Goal
- Replace legal route placeholders with concrete, readable policy pages for Privacy Policy, Terms of Service, and Service Level Agreement.
- Ensure legal pages are first-class rendered routes in static output and no longer show "Generating" placeholder states.

## Non-goals
- No claim of legal counsel or jurisdiction-specific compliance certification.
- No backend/API behavior changes.
- No changes to non-legal site routes.

## Security and constraints
- Keep legal content deterministic and versioned in repo.
- Use existing frontend layout patterns and avoid adding runtime dependencies.
- Remove dead placeholder-only legal component if no longer referenced.

## Acceptance criteria
- `/legal/privacy/`, `/legal/terms/`, and `/legal/sla/` render substantive policy text.
- Pages include clear effective date and policy scope statements.
- Pages do not render "Generating" badge/indicator or placeholder copy.
- Frontend check/build pass and Cypress assertions verify legal page content.

## Rollout and rollback
- Rollout: merge legal page content and test coverage in one change.
- Rollback: restore previous placeholder pages if legal copy must be revised urgently.
