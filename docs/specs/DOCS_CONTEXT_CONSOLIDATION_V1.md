# Docs Context Consolidation Spec (V1)

## Status
- Active
- Date: 2026-02-23
- Owner: repo operations

## Goal
Reduce documentation sprawl into a compact canonical context set that can guide day-to-day execution decisions without hunting across dozens of files.

## Non-goals
- Deleting component-level API/protocol docs that are still needed for implementation detail.
- Changing runtime behavior, protocol contracts, or deployment targets.
- Replacing legal notices/licenses/trademark documents.

## Security and Constraint Requirements
- Preserve all non-negotiable constraints from `AGENTS.md` as top-level operating rules.
- Preserve deterministic/runtime constraints from whitepapers and protocol specs.
- Preserve execution-proof requirements (checks/tests/evidence) as mandatory quality gates.
- Preserve architecture boundaries (`frontend/` canonical root, storage boundary, protocol-first contracts).

## Acceptance Criteria
1. Two canonical compact docs exist and are sufficient for operator mentality + execution workflow:
   - repo operating context (principles, boundaries, priorities, constraints)
   - executable guidelines (checklists, commands, evidence format, escalation rules)
2. Compact docs include mapped coverage across governance, specs, proposals, frontend/cloud docs, scripts, and app READMEs.
3. Compact docs are explicit about canonical vs legacy references.
4. Validation commands run and results are reported.

## Rollout
1. Create this spec.
2. Add compact docs with dense synthesis and actionable checklists.
3. Run validation commands.
4. Report evidence and identify remaining cleanup step options.

## Rollback
- Remove compact docs and this spec file if consolidation direction is rejected.
- Continue using existing doc set unchanged.
