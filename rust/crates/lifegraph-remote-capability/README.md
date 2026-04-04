# lifegraph-remote-capability

Remote capability transport helpers, adapters, and demo binaries for Lifegraph capability sessions.

## Demo binaries

This crate now includes a simple socket-framed demo pair:

- `capability-demo-server`
- `capability-demo-client`

They speak length-prefixed `CapabilityRemoteEnvelope` frames encoded with protobuf.

## TCP demo

Start the server:

```bash
cd rust
cargo run -q -p lifegraph-remote-capability --bin capability-demo-server -- tcp 127.0.0.1:47070
```

In another terminal, connect the client:

```bash
cd rust
cargo run -q -p lifegraph-remote-capability --bin capability-demo-client -- tcp 127.0.0.1:47070
```

Expected shape:

- client stderr prints a `SessionAccept`
- client stdout prints a successful `ResultFrame`
- inline payload in the result is `demo-ok`

## Unix socket demo

Start the server:

```bash
cd rust
cargo run -q -p lifegraph-remote-capability --bin capability-demo-server -- unix /tmp/lifegraph-cap-demo.sock
```

Client:

```bash
cd rust
cargo run -q -p lifegraph-remote-capability --bin capability-demo-client -- unix /tmp/lifegraph-cap-demo.sock
```

## Transport model

`FramedRemoteTransport<S>` supports:

- Unix domain sockets
- TCP streams

Frame format:

- 4-byte big-endian payload length
- protobuf-encoded `CapabilityRemoteEnvelope`

## Current demo flow

The demo server exercises the real policy-wrapped session path:

1. `SessionOpen`
2. normalized `CapabilityRequest`
3. policy-issued `CapabilityGrant`
4. `SessionAccept`
5. `Invocation`
6. `ResultFrame`

So the session transport path is aligned with the proto request/grant model.
