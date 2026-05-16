# edgerun-remote-capability

Remote capability protocol helpers and adapters for Edgerun capability sessions.

## Boundary

This crate does not bind sockets, open ports, or decide transport ownership.
The node owns those resources and hands already-authorized streams to capability
session handlers. When a capability session crosses a node, relay, or runtime
boundary, invocation authority should be expressible as admitted work; this
crate only carries the session/envelope adapter shape.

## Framed transport

`FramedRemoteTransport<S>` supports:

- any stream already owned by the caller that implements `Read + Write`

Frame format:

- 4-byte big-endian payload length
- rkyv-encoded `CapabilityRemoteEnvelope`

## Session flow

The local policy-wrapped session path is:

1. `SessionOpen`
2. normalized `CapabilityRequest`
3. policy-issued `CapabilityGrant`
4. `SessionAccept`
5. `Invocation`
6. `ResultFrame`

The session transport path is aligned with the proto request/grant model.
Standalone inbound `Grant` envelopes are rejected: a remote peer cannot import
authority into a provider. Local grants are projections of provider policy and
must be bound to a `SessionOpen`/`CapabilityRequest`; cross-boundary capability
authority should additionally bind to `edgerun-work` admission, route, and
capability packet evidence. Invocations must be bound to an accepted session.
