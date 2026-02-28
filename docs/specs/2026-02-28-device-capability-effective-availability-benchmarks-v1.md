# Device Capability Effective Availability Benchmarks V1

## Goal
- Define a deterministic benchmark suite that converts detected capability signals into effective availability values for scheduler decisions.
- Standardize benchmark execution, scoring, and output across Linux, Android, Windows, and constrained router-class devices.
- Keep benchmarks performance-only; capability presence checks are sourced from capability probes, not benchmark cases.

## Non-goals
- No vendor-specific synthetic benchmark tuning in V1.
- No destructive stress tests that risk node stability.
- No scheduler integration logic in this phase.

## Security and constraints
- Benchmarks must be safe-by-default, read-only where possible, and bounded by strict timeouts.
- Benchmarks must avoid privileged escalation and must report permission blockers explicitly.
- All benchmark artifacts must be written under `out/`.

## Benchmark output contract
- Each benchmark emits:
  - `domain`: capability domain id
  - `case`: benchmark case id
  - `status`: `pass|degraded|fail|blocked|unknown`
  - `score_milli`: normalized 0..1000
  - `duration_ms`
  - `sample_count`
  - `error_code` (if any)
  - `source_path_or_api`
- Aggregate output emits:
  - `effective_availability_milli` per domain
  - `confidence` per domain
  - `blockers` list
  - `collected_unix_s`, `ttl_s`
- Artifact format:
  - `protobuf` binary (`report.pb`) for contract consumption.

## Benchmark matrix

### CPU
1. `cpu.compute.int`
- Method: bounded integer workload over fixed iterations.
- Score: operations/sec normalized to baseline profile.
- Maps to: `domains.cpu.available` + `metrics.cpu_load_ratio_per_core_milli`.

2. `cpu.compute.float`
- Method: bounded floating-point workload.
- Score: throughput normalized.

3. `cpu.latency.scheduling`
- Method: timer wakeup jitter sample.
- Score: inverse p95 jitter.

### RAM
1. `ram.bandwidth.copy`
- Method: bounded memory copy/read-write sweep.
- Score: MiB/s normalized.

2. `ram.headroom`
- Method: verify allocatable bytes against requested floor.
- Score: available/required ratio, capped.

### Storage
1. `storage.seq.read`
- Method: bounded sequential read on writable test area (`out/bench/tmp`).
- Score: MiB/s normalized.

2. `storage.seq.write`
- Method: bounded sequential write with fsync policy marker.
- Score: MiB/s normalized with penalty for high latency.

3. `storage.latency`
- Method: small random IO sample.
- Score: inverse p95 latency.

### Network
1. `network.loopback.rtt`
- Method: UDP loopback ping-pong p95 latency sample.
- Score: inverse latency band score.

2. `network.loopback.throughput`
- Method: bounded TCP loopback burst transfer.
- Score: throughput band score.

3. `network.egress.rtt` (optional)
- Method: configurable control-plane endpoint TCP/UDP handshake latency (no heavy traffic).
- Score: inverse p95 latency.

4. `network.throughput.smallburst` (optional)
- Method: bounded burst transfer to test endpoint.
- Score: throughput normalized.

### GPU
1. `gpu.compute.smoke` (deferred)
- Method: short shader/compute workload with explicit backend selection.
- Score: throughput/latency band score.

2. `gpu.mem.transfer` (deferred)
- Method: bounded host-device transfer sample.
- Score: bandwidth band score.

#### GPU V1 implementation note
- GPU API/runtime availability remains in capability probe outputs (`detected/available/in_use`) and is not benchmark-scored.
- GPU performance benchmarks are deferred until a cross-platform context/device harness is available.

### USB
1. `usb.enumeration`
- Method: enumerate usb bus entries and access rights.
- Score: count + permission weighting.

2. `usb.io.smoke` (optional)
- Method: only when approved test device is configured.
- Score: pass/degraded/fail.

### Input
1. `input.enumeration`
- Method: enumerate input nodes/classes.
- Score: count + readability.

2. `input.event.readiness` (optional)
- Method: non-blocking readiness probe.
- Score: pass/degraded/fail.

### Output
1. `output.enumeration`
- Method: enumerate display/fb/tty outputs.
- Score: count + permission weighting.

2. `output.present`
- Method: display/console availability check.
- Score: pass/degraded/fail.

### Bluetooth
1. `bluetooth.enumeration`
- Method: adapter/service presence and access check.
- Score: count + permission weighting.

2. `bluetooth.radio.ready` (optional)
- Method: stack-ready state probe (no pairing).
- Score: pass/degraded/fail.

### NFC
1. `nfc.enumeration`
- Method: adapter/service presence and access check.
- Score: count + permission weighting.

2. `nfc.reader.ready` (optional)
- Method: non-invasive reader readiness check.
- Score: pass/degraded/fail.

## Policy-aware effective availability mapping
- Domain effective availability is computed from:
  - `detection_score`
  - `permission_score`
  - `performance_score`
  - `freshness_score`
- Suggested weights:
  - detection: 0.20
  - permission: 0.30
  - performance: 0.40
  - freshness: 0.10
- Rule overrides:
  - If permission is blocked, final availability = `Unsupported`.
  - If benchmark blocked/unknown for mandatory case, final availability = `Unknown` with blocker.

## Recommended thresholds
- `>= 800`: `Supported` (high confidence)
- `600..799`: `Supported` (medium confidence / degraded)
- `300..599`: `Unsupported` for performance-critical jobs, `Supported` for best-effort jobs
- `< 300`: `Unsupported`

## Runtime profiles
1. `router-lite` (MT7621/low-power)
- Short runs, lower baseline targets, strict thermal guard.

2. `edge-standard`
- Balanced duration and confidence.

3. `edge-performance`
- Higher sample count and stronger confidence requirements.

## Acceptance criteria
- Benchmark spec and output contract are documented.
- Each capability domain has at least one mandatory benchmark case.
- Benchmark outputs can be deterministically transformed into effective availability fields in capability report.
- Missing benchmark execution paths are represented explicitly as blockers or unknowns.

## Rollout
1. Implement benchmark runner in host crate or companion crate.
2. Emit protobuf artifacts to `out/bench/`.
3. Integrate transformation into capability report generation.
4. Gate scheduler integration on benchmark coverage + confidence.

## Rollback
- Disable benchmark-derived overrides and fall back to current capability-only availability model.
