# 2026-03-01 NATS JetStream Central Event Bus And Container Agents V1

## Goal
- Make the event bus the central communication channel across components and integrations.
- Introduce NATS JetStream as the shared transport for topic-based communication.
- Establish a deterministic containerized parallel-agent workflow where each agent works in an isolated codebase copy.

## Non-Goals
- Full migration of every runtime service to NATS in this single change.
- Replacing all existing local edge-internal gRPC wiring immediately.
- Implementing cloud multi-region message replication policy in this phase.

## Security And Constraints
- Keep fail-closed local behavior when NATS is unavailable.
- No ports/services on `8080`.
- Preserve topic scoping (`edge_internal`, `edge_cluster`) and do not allow unscoped wildcard writes from integrations.
- Agent containers must use isolated git worktrees and branch names for merge traceability.

## Design
1. Event bus bridge:
   - Extend `edgerun-event-bus` with optional NATS JetStream mirroring.
   - On edge-internal publish, mirror envelopes to NATS JetStream subjects.
   - Subscribe to mirrored subjects and inject remote envelopes into local runtime bus.
   - Prevent loopback replay using per-instance origin identifiers.
2. Transport conventions:
   - Subject root: `edgerun.events`.
   - Overlay mapping:
     - `edge_internal` -> `edgerun.events.edge_internal.<topic>`
     - `edge_cluster` -> `edgerun.events.edge_cluster.<topic>`
3. Compose foundation:
   - Add NATS service with JetStream enabled to local stack.
4. Parallel container agent workflow:
   - `scripts/agents/launch-agent.sh`: create isolated git worktree and run containerized agent execution in that workspace.
   - `scripts/agents/merge-agent.sh`: merge agent branch into target branch deterministically.

## Acceptance Criteria
1. `edgerun-event-bus` compiles with NATS JetStream bridge support.
2. Local compose stack includes NATS JetStream service and health probe.
3. Agent workflow scripts create isolated worktree per agent id and run containerized execution there.
4. Agent merge script merges from agent branch to target branch non-interactively.
5. Validation commands for touched scope pass.

## Rollout
- Enable NATS via env vars where desired; default remains local-only behavior.
- Bring up stack with NATS service and incrementally opt-in services to NATS bridge.
- Move integrations/agents to topic-only communication as subsequent phases.

## Rollback
- Disable NATS bridge env vars and continue with local runtime bus only.
- Remove NATS service from compose profile if required.
- Revert agent scripts without affecting existing runtime paths.
