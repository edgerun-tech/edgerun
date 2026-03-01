# 2026-03-01 Segmented Journal Append Rollover V1

## Goal
- Eliminate per-event segment creation+seal overhead in `append_event_to_segmented_journal`.
- Reuse current segment for appends and roll over only when full.

## Non-Goals
- Changing event format.
- Background compaction/seal policy redesign.

## Constraints
- Append-only behavior preserved.
- Segment registry stays authoritative for query ordering.
- On `SegmentError::Full`, append retries exactly once on a newly created segment.

## Design
1. If registry has segments, append to last segment file.
2. If registry is empty, create first segment and persist registry.
3. Append via `EngineAppendSession` without sealing per event.
4. On full segment error, create next segment part, persist registry, append there.

## Acceptance Criteria
1. Multiple appends go to same segment until full.
2. Queries return all events via registry in order.
3. Existing tests pass and new regression test validates segment reuse.

## Rollback
- Revert to prior one-event-per-segment behavior if required.
