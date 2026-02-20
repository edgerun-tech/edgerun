<!-- SPDX-License-Identifier: GPL-2.0-only -->
# Architecture

## Write Path

1. Event encode
2. In-memory index updates
3. Segment append buffering
4. Centralized `io_uring` reactor submission
5. Durable flush policy enforcement

## I/O Model

- Single process-wide `io_uring` reactor.
- Dedicated reactor thread owns ring submission/completion.
- Worker threads communicate via channels/tickets.

## Durability Modes

- `AckLocal` / `AckBuffered`: local memory/buffered progression.
- `AckDurable`: write + fsync durability.
- `AckCheckpointed`: linked write/fsync + manifest write/fsync chain.
- `AckReplicatedN`: local durable write + quorum remote ACKs.

## Replication Transport

- Supports single-op and multi-op ACK frames.
- Authenticated ACK v2 mode available.
- Pooled long-lived connections and retry/backoff semantics.
- Group-commit APIs:
  - `append_batch_with_durability`
  - `append_stream_with_durability`
  - `append_replicated_stream`

## Validation Strategy

- Unit + integration tests
- Crash campaign (`crash_campaign`)
- Perf gates (`tools/perf_gate.sh`)
- Mixed workload tuning sweep (`tools/mixed_rw_tuning_sweep.sh`)
