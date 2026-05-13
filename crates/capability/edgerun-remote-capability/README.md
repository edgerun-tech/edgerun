# edgerun-remote-capability

Remote capability protocol helpers and adapters for Edgerun capability sessions.

## Boundary

This crate does not bind sockets, open ports, or decide transport ownership.
The node owns those resources and hands already-authorized streams to capability
session handlers.

## Framed transport

`FramedRemoteTransport<S>` supports:

- any stream already owned by the caller that implements `Read + Write`

Frame format:

- 4-byte big-endian payload length
- rkyv-encoded `CapabilityRemoteEnvelope`

## Session flow

The policy-wrapped session path is:

1. `SessionOpen`
2. normalized `CapabilityRequest`
3. policy-issued `CapabilityGrant`
4. `SessionAccept`
5. `Invocation`
6. `ResultFrame`

The session transport path is aligned with the proto request/grant model.
Standalone inbound `Grant` envelopes are rejected: a remote peer cannot import
authority into a provider. Grants are created by the provider policy as part of
`SessionOpen`/`CapabilityRequest` admission, and invocations must be bound to an
accepted session.
