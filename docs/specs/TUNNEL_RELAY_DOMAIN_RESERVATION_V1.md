# Tunnel Relay Domain Reservation V1

## Goal
- Enable user domain reservation tied to profile public key.
- Provide ngrok-style outbound tunnel registration flow for nodes.
- Provide one-time pairing code flow so control plane can show exact device command.
- Deliver first local-first implementation slice with explicit control-plane contracts.

## Non-goals
- No production-grade global relay dataplane implementation in this slice.
- No DNS automation or wildcard cert provisioning in this slice.
- No durable database; in-memory registry only.

## Security and constraints
- Domain reservation is fail-closed: malformed request or missing profile public key is rejected.
- Tunnel registration requires pre-reserved domain, explicit node identity fields, and a reservation token issued at reserve-time.
- Relay verifies endpoint registration signature (ed25519) over a canonical registration message.
- Relay can mint one-time pairing codes bound to a reserved domain; device can register with pairing code only.
- Session lifecycle has bounded TTL and heartbeat extension.
- All control messages use protobuf wire format (`application/protobuf`).

## Acceptance criteria
- Tunnel control protobuf schema exists under `edgerun-runtime-proto` and compiles.
- New relay control binary crate exists with endpoints:
  - reserve domain,
  - register endpoint,
  - heartbeat endpoint.
- `edgerun-node-manager` has tunnel commands:
  - reserve domain,
  - register endpoint,
  - create pairing code,
  - connect with pairing code,
  - heartbeat.
- Commands and service exchange protobuf requests/responses end-to-end.
- Rust workspace checks pass.

## Rollout and rollback
- Rollout: additive proto + relay service + node-manager commands.
- Rollback: remove relay crate from workspace and revert node-manager tunnel command additions.
